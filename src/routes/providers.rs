use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde_json::json;
use tracing::{info, debug};
use uuid::Uuid;

use crate::{
    db::{
        create_cloud_provider, delete_cloud_provider, get_cloud_provider_by_id,
        list_cloud_providers, test_cloud_provider_connectivity, update_cloud_provider,
    },
    models::{CloudProviderType, ConnectivityTestResult, NewCloudProvider, UpdateCloudProvider},
    AppError, AppState,
};

/// Lista todos os provedores cloud configurados
/// 
/// Retorna uma lista de todos os provedores de armazenamento cloud cadastrados,
/// incluindo informações de status e conectividade.
/// 
/// # Retorna
/// * `Ok(Json<Vec<CloudProvider>>)` - Lista de provedores
/// * `Err(AppError)` - Erro de banco de dados
#[utoipa::path(
    get,
    path = "/providers",
    tag = "Cloud Providers",
    responses(
        (status = 200, description = "Lista de provedores cloud", body = [crate::models::CloudProvider]),
        (status = 500, description = "Erro interno", body = crate::models::ErrorResponse)
    )
)]
pub async fn list_providers(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    debug!("Listando provedores cloud");
    
    let providers = list_cloud_providers(&state.db_pool).await?;
    
    info!(count = providers.len(), "Provedores cloud listados com sucesso");
    Ok((StatusCode::OK, Json(providers)))
}

/// Cria um novo provedor cloud
/// 
/// Registra um novo provedor de armazenamento cloud com suas credenciais e configurações.
/// Opcionalmente testa a conectividade após a criação.
/// 
/// # Argumentos
/// * `payload` - Dados do novo provedor
/// 
/// # Retorna
/// * `Ok(Json<CloudProvider>)` - Provedor criado
/// * `Err(AppError)` - Erro de validação ou banco de dados
#[utoipa::path(
    post,
    path = "/providers",
    tag = "Cloud Providers",
    request_body = NewCloudProvider,
    responses(
        (status = 201, description = "Provedor criado com sucesso", body = crate::models::CloudProvider),
        (status = 400, description = "Dados inválidos", body = crate::models::ErrorResponse),
        (status = 500, description = "Erro interno", body = crate::models::ErrorResponse)
    )
)]
pub async fn create_provider(
    State(state): State<AppState>,
    Json(payload): Json<NewCloudProvider>,
) -> Result<impl IntoResponse, AppError> {
    info!(
        name = %payload.name,
        provider_type = ?payload.provider_type,
        bucket = %payload.bucket,
        "Criando novo provedor cloud"
    );

    // Validações específicas por tipo de provedor
    match payload.provider_type {
        CloudProviderType::BackblazeB2 => {
            // Para B2, validar se tem as credenciais corretas dependendo do tipo de API
            if payload.use_b2_native_api.unwrap_or(false) {
                if payload.b2_account_id.is_none() || payload.b2_application_key.is_none() {
                    return Err(AppError::BadRequest(
                        "B2 native API requires b2_account_id and b2_application_key".to_string(),
                    ));
                }
            }
        }
        CloudProviderType::IdriveE2 => {
            if payload.endpoint.is_none() {
                return Err(AppError::BadRequest(
                    "IDrive e2 requires endpoint URL".to_string(),
                ));
            }
        }
        CloudProviderType::Wasabi => {
            if payload.region.is_none() {
                return Err(AppError::BadRequest(
                    "Wasabi requires region specification".to_string(),
                ));
            }
        }
        CloudProviderType::Scaleway => {
            if payload.region.is_none() {
                return Err(AppError::BadRequest(
                    "Scaleway requires region specification".to_string(),
                ));
            }
        }
    }

    let provider = create_cloud_provider(&state.db_pool, &payload).await?;

    info!(
        provider_id = %provider.id,
        name = %provider.name,
        "Provedor cloud criado com sucesso"
    );

    Ok((StatusCode::CREATED, Json(provider)))
}

/// Obtém detalhes de um provedor específico
/// 
/// Retorna as informações completas de um provedor cloud específico,
/// excluindo credenciais sensíveis.
/// 
/// # Argumentos
/// * `id` - UUID do provedor
/// 
/// # Retorna
/// * `Ok(Json<CloudProvider>)` - Dados do provedor
/// * `Err(AppError)` - Provedor não encontrado ou erro de banco
#[utoipa::path(
    get,
    path = "/providers/{id}",
    tag = "Cloud Providers",
    params(
        ("id" = Uuid, Path, description = "ID do provedor cloud")
    ),
    responses(
        (status = 200, description = "Detalhes do provedor", body = crate::models::CloudProvider),
        (status = 404, description = "Provedor não encontrado", body = crate::models::ErrorResponse),
        (status = 500, description = "Erro interno", body = crate::models::ErrorResponse)
    )
)]
pub async fn get_provider(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    debug!(provider_id = %id, "Buscando provedor cloud específico");

    let provider = get_cloud_provider_by_id(&state.db_pool, id).await?
        .ok_or_else(|| AppError::NotFound(format!("Provider with id {} not found", id)))?;

    info!(
        provider_id = %id,
        name = %provider.name,
        "Provedor cloud encontrado"
    );

    Ok((StatusCode::OK, Json(provider)))
}

/// Atualiza configurações de um provedor
/// 
/// Permite atualizar configurações de um provedor existente, incluindo
/// credenciais, endpoints e outras configurações.
/// 
/// # Argumentos
/// * `id` - UUID do provedor
/// * `payload` - Dados para atualização
/// 
/// # Retorna
/// * `Ok(Json<CloudProvider>)` - Provedor atualizado
/// * `Err(AppError)` - Provedor não encontrado ou erro de banco
#[utoipa::path(
    put,
    path = "/providers/{id}",
    tag = "Cloud Providers",
    params(
        ("id" = Uuid, Path, description = "ID do provedor cloud")
    ),
    request_body = UpdateCloudProvider,
    responses(
        (status = 200, description = "Provedor atualizado", body = crate::models::CloudProvider),
        (status = 404, description = "Provedor não encontrado", body = crate::models::ErrorResponse),
        (status = 500, description = "Erro interno", body = crate::models::ErrorResponse)
    )
)]
pub async fn update_provider(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateCloudProvider>,
) -> Result<impl IntoResponse, AppError> {
    debug!(
        provider_id = %id,
        update_fields = ?payload,
        "Atualizando provedor cloud"
    );

    let provider = update_cloud_provider(&state.db_pool, id, &payload).await?;

    info!(
        provider_id = %id,
        name = %provider.as_ref().map(|p| &p.name).unwrap_or(&"unknown".to_string()),
        "Provedor cloud atualizado com sucesso"
    );

    Ok((StatusCode::OK, Json(provider)))
}

/// Remove um provedor cloud
/// 
/// Remove permanentemente um provedor da configuração. Esta ação não pode ser desfeita.
/// Todos os backups associados a este provedor devem ser migrados antes da remoção.
/// 
/// # Argumentos
/// * `id` - UUID do provedor
/// 
/// # Retorna
/// * `Ok(())` - Provedor removido com sucesso
/// * `Err(AppError)` - Provedor não encontrado ou erro de banco
#[utoipa::path(
    delete,
    path = "/providers/{id}",
    tag = "Cloud Providers",
    params(
        ("id" = Uuid, Path, description = "ID do provedor cloud")
    ),
    responses(
        (status = 204, description = "Provedor removido com sucesso"),
        (status = 404, description = "Provedor não encontrado", body = crate::models::ErrorResponse),
        (status = 409, description = "Provedor em uso", body = crate::models::ErrorResponse),
        (status = 500, description = "Erro interno", body = crate::models::ErrorResponse)
    )
)]
pub async fn delete_provider(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    debug!(provider_id = %id, "Removendo provedor cloud");

    // TODO: Verificar se o provedor não está sendo usado por algum backup job
    // Isso deve ser implementado como uma foreign key constraint ou verificação manual

    delete_cloud_provider(&state.db_pool, id).await?;

    info!(provider_id = %id, "Provedor cloud removido com sucesso");

    Ok(StatusCode::NO_CONTENT)
}

/// Testa conectividade com um provedor
/// 
/// Executa um teste de conectividade com o provedor cloud especificado,
/// validando credenciais e acesso ao bucket.
/// 
/// # Argumentos
/// * `id` - UUID do provedor
/// 
/// # Retorna
/// * `Ok(Json<ConnectivityTestResult>)` - Resultado do teste
/// * `Err(AppError)` - Provedor não encontrado ou erro de teste
#[utoipa::path(
    post,
    path = "/providers/{id}/test",
    tag = "Cloud Providers",
    params(
        ("id" = Uuid, Path, description = "ID do provedor cloud")
    ),
    responses(
        (status = 200, description = "Teste realizado", body = ConnectivityTestResult),
        (status = 404, description = "Provedor não encontrado", body = crate::models::ErrorResponse),
        (status = 500, description = "Erro interno", body = crate::models::ErrorResponse)
    )
)]
pub async fn test_provider_connectivity(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    debug!(provider_id = %id, "Testando conectividade do provedor cloud");

    let test_result = test_cloud_provider_connectivity(&state.db_pool, id).await?;

    info!(
        provider_id = %id,
        success = test_result.success,
        status = ?test_result.status,
        "Teste de conectividade concluído"
    );

    Ok((StatusCode::OK, Json(test_result)))
}

/// Lista tipos de provedores suportados
/// 
/// Retorna informações sobre os tipos de provedores cloud suportados
/// pelo sistema, incluindo seus requisitos de configuração.
/// 
/// # Retorna
/// * `Ok(Json<Value>)` - Lista de tipos suportados
#[utoipa::path(
    get,
    path = "/providers/types",
    tag = "Cloud Providers",  
    responses(
        (status = 200, description = "Tipos de provedores suportados")
    )
)]
pub async fn list_provider_types() -> Result<impl IntoResponse, AppError> {
    debug!("Listando tipos de provedores cloud suportados");

    let types = json!({
        "supported_types": [
            {
                "type": "backblaze_b2",
                "name": "Backblaze B2",
                "description": "Armazenamento cloud focado em backup com boa relação custo-benefício",
                "supports_s3_api": true,
                "supports_native_api": true,
                "required_fields": ["access_key", "secret_key", "bucket"],
                "optional_fields": ["b2_account_id", "b2_application_key", "use_b2_native_api"],
                "pricing": {
                    "storage_per_tb": 6.0,
                    "egress_per_tb": 10.0,
                    "currency": "USD"
                }
            },
            {
                "type": "idrive_e2",
                "name": "IDrive e2", 
                "description": "Mais barato com egress gratuito",
                "supports_s3_api": true,
                "supports_native_api": false,
                "required_fields": ["access_key", "secret_key", "bucket", "endpoint"],
                "optional_fields": ["region"],
                "pricing": {
                    "storage_per_tb": 4.0,
                    "egress_per_tb": 0.0,
                    "currency": "USD"
                }
            },
            {
                "type": "wasabi",
                "name": "Wasabi",
                "description": "Performance alta com egress gratuito (até limites)",
                "supports_s3_api": true,
                "supports_native_api": false,
                "required_fields": ["access_key", "secret_key", "bucket", "region"],
                "optional_fields": ["endpoint"],
                "pricing": {
                    "storage_per_tb": 7.0,
                    "egress_per_tb": 0.0,
                    "currency": "USD",
                    "notes": "Egress gratuito até 100% do storage mensal"
                }
            },
            {
                "type": "scaleway",
                "name": "Scaleway Object Storage",
                "description": "GDPR compliant, baseado na Europa",
                "supports_s3_api": true,
                "supports_native_api": false,
                "required_fields": ["access_key", "secret_key", "bucket", "region"],
                "optional_fields": ["endpoint"],
                "pricing": {
                    "storage_per_tb": 7.5,
                    "egress_per_tb": 10.0,
                    "currency": "EUR"
                }
            }
        ],
        "generated_at": Utc::now()
    });

    Ok((StatusCode::OK, Json(types)))
}

/// Retorna templates de configuração para cada provedor
/// 
/// Fornece exemplos práticos de como configurar cada tipo de provedor,
/// incluindo URLs de onde obter credenciais e exemplos reais.
/// 
/// # Retorna
/// * `Ok(Json<Value>)` - Templates de configuração com exemplos
#[utoipa::path(
    get,
    path = "/providers/templates",
    tag = "Cloud Providers",  
    responses(
        (status = 200, description = "Templates de configuração para cada provedor")
    )
)]
pub async fn get_provider_templates() -> Result<impl IntoResponse, AppError> {
    debug!("Fornecendo templates de configuração dos provedores");

    let templates = json!({
        "templates": [
            {
                "provider_type": "backblaze_b2",
                "name": "Backblaze B2",
                "description": "Armazenamento cloud focado em backup com boa relação custo-benefício",
                "signup_url": "https://www.backblaze.com/b2/cloud-storage.html",
                "pricing": "$6/TB storage + $10/TB egress",
                "configurations": [
                    {
                        "name": "S3-Compatible API (Recomendado)",
                        "description": "Usa endpoint S3 do Backblaze - mais compatível",
                        "setup_steps": [
                            "1. Crie uma conta em https://www.backblaze.com",
                            "2. Vá em App Keys e crie uma Application Key",
                            "3. Anote o keyID (access_key) e applicationKey (secret_key)",
                            "4. Encontre seu endpoint S3 em Account > Buckets"
                        ],
                        "example": {
                            "name": "Meu Backup B2",
                            "provider_type": "backblaze_b2",
                            "endpoint": "https://s3.us-west-002.backblazeb2.com",
                            "region": "us-west-002",
                            "bucket": "meu-bucket-backup",
                            "access_key": "seu_keyID_aqui",
                            "secret_key": "sua_applicationKey_aqui",
                            "use_b2_native_api": false
                        }
                    },
                    {
                        "name": "B2 Native API",
                        "description": "API nativa do B2 - recursos específicos",
                        "example": {
                            "name": "B2 Native",
                            "provider_type": "backblaze_b2",
                            "bucket": "meu-bucket",
                            "access_key": "seu_keyID",
                            "secret_key": "sua_applicationKey",
                            "b2_account_id": "seu_account_id",
                            "b2_application_key": "sua_app_key",
                            "use_b2_native_api": true
                        }
                    }
                ]
            },
            {
                "provider_type": "idrive_e2",
                "name": "IDrive e2",
                "description": "Melhor custo-benefício: $4/TB + egress GRÁTIS",
                "signup_url": "https://www.idrive.com/e2/",
                "pricing": "$4/TB storage + $0 egress (grátis!)",
                "configuration": {
                    "setup_steps": [
                        "1. Crie conta em https://www.idrive.com/e2/",
                        "2. Vá em Access Keys e crie uma nova chave",
                        "3. Anote Access Key ID e Secret Access Key",
                        "4. Endpoint será fornecido na dashboard"
                    ],
                    "example": {
                        "name": "IDrive e2 Backup",
                        "provider_type": "idrive_e2",
                        "endpoint": "https://endpoint.idrivee2.com",
                        "region": "us-west",
                        "bucket": "meu-bucket-backup",
                        "access_key": "sua_access_key_id",
                        "secret_key": "sua_secret_access_key"
                    },
                    "common_endpoints": [
                        "https://endpoint.idrivee2.com (US West)",
                        "https://eu.endpoint.idrivee2.com (Europe)",
                        "https://ap.endpoint.idrivee2.com (Asia Pacific)"
                    ]
                }
            },
            {
                "provider_type": "wasabi",
                "name": "Wasabi Hot Cloud Storage",
                "description": "Performance alta + egress gratuito até limites",
                "signup_url": "https://wasabi.com/",
                "pricing": "$7/TB storage + egress grátis até 100% do storage/mês",
                "configuration": {
                    "setup_steps": [
                        "1. Crie conta em https://wasabi.com",
                        "2. Vá em Access Keys e crie nova chave",
                        "3. Escolha a região mais próxima",
                        "4. Anote Access Key e Secret Key"
                    ],
                    "example": {
                        "name": "Wasabi Backup",
                        "provider_type": "wasabi",
                        "region": "us-east-1",
                        "bucket": "meu-bucket-backup",
                        "access_key": "sua_access_key",
                        "secret_key": "sua_secret_key"
                    },
                    "available_regions": [
                        "us-east-1 (Virginia)",
                        "us-east-2 (Virginia)",
                        "us-west-1 (Oregon)",
                        "eu-central-1 (Amsterdam)",
                        "ap-northeast-1 (Tokyo)",
                        "ap-northeast-2 (Osaka)"
                    ]
                }
            },
            {
                "provider_type": "scaleway",
                "name": "Scaleway Object Storage",
                "description": "GDPR compliant, baseado na Europa",
                "signup_url": "https://www.scaleway.com/en/object-storage/",
                "pricing": "€7.5/TB storage + €10/TB egress",
                "configuration": {
                    "setup_steps": [
                        "1. Crie conta em https://console.scaleway.com",
                        "2. Vá em API Keys e gere nova chave",
                        "3. Escolha região europeia",
                        "4. Configure Object Storage"
                    ],
                    "example": {
                        "name": "Scaleway EU Backup",
                        "provider_type": "scaleway",
                        "endpoint": "https://s3.fr-par.scw.cloud",
                        "region": "fr-par",
                        "bucket": "meu-bucket-backup",
                        "access_key": "sua_access_key",
                        "secret_key": "sua_secret_key"
                    },
                    "available_regions": [
                        "fr-par (Paris, France)",
                        "nl-ams (Amsterdam, Netherlands)",
                        "pl-waw (Warsaw, Poland)"
                    ]
                }
            }
        ],
        "general_tips": [
            "Sempre teste a conectividade após configurar: POST /providers/{id}/test",
            "IDrive e2 tem o melhor custo total (egress gratuito)",
            "Backblaze B2 é o mais maduro e confiável",
            "Wasabi tem melhor performance global",
            "Scaleway é ideal para compliance europeu (GDPR)"
        ],
        "generated_at": Utc::now()
    });

    Ok((StatusCode::OK, Json(templates)))
}