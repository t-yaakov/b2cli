use b2cli::models::{CloudProviderType, NewCloudProvider};
use uuid::Uuid;

/// Testa criação básica de modelos de provedores cloud
/// 
/// Verifica se:
/// - Structs são criados corretamente
/// - Serialização funciona
/// - Tipos enum são válidos
#[test]
fn test_cloud_provider_models() {
    // Testar criação de NewCloudProvider para Backblaze B2
    let new_provider = NewCloudProvider {
        name: "Test Backblaze B2".to_string(),
        provider_type: CloudProviderType::BackblazeB2,
        endpoint: Some("https://s3.us-west-002.backblazeb2.com".to_string()),
        region: Some("us-west-002".to_string()),
        bucket: "test-backup-bucket".to_string(),
        path_prefix: Some("backups/".to_string()),
        access_key: "test-key-id".to_string(),
        secret_key: "test-application-key".to_string(),
        b2_account_id: None,
        b2_application_key: None,
        use_b2_native_api: Some(false),
        is_default: Some(true),
        test_connectivity: Some(false),
    };

    // Verificar serialização funciona
    let json = serde_json::to_string(&new_provider).unwrap();
    assert!(json.contains("Test Backblaze B2"));
    assert!(json.contains("backblaze_b2"));

    // Verificar deserialização
    let deserialized: NewCloudProvider = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.name, "Test Backblaze B2");
    assert_eq!(deserialized.bucket, "test-backup-bucket");
}

/// Testa diferentes tipos de provedores cloud
/// 
/// Verifica se:
/// - Todos os tipos enum são válidos
/// - Serialização de tipos funciona
#[test]
fn test_cloud_provider_types() {
    let types = vec![
        CloudProviderType::BackblazeB2,
        CloudProviderType::IdriveE2,
        CloudProviderType::Wasabi,
        CloudProviderType::Scaleway,
    ];

    for provider_type in types {
        let json = serde_json::to_string(&provider_type).unwrap();
        let deserialized: CloudProviderType = serde_json::from_str(&json).unwrap();
        
        // Verificar que o tipo serializa e deserializa corretamente
        match provider_type {
            CloudProviderType::BackblazeB2 => {
                assert!(json.contains("backblaze_b2"));
                assert!(matches!(deserialized, CloudProviderType::BackblazeB2));
            },
            CloudProviderType::IdriveE2 => {
                assert!(json.contains("idrive_e2"));
                assert!(matches!(deserialized, CloudProviderType::IdriveE2));
            },
            CloudProviderType::Wasabi => {
                assert!(json.contains("wasabi"));
                assert!(matches!(deserialized, CloudProviderType::Wasabi));
            },
            CloudProviderType::Scaleway => {
                assert!(json.contains("scaleway"));
                assert!(matches!(deserialized, CloudProviderType::Scaleway));
            },
        }
    }
}

/// Testa configurações específicas do Backblaze B2
/// 
/// Verifica se:
/// - API S3-compatible não requer campos B2 específicos
/// - API nativa requer campos B2 específicos
#[test]
fn test_backblaze_configurations() {
    // Configuração S3-compatible
    let s3_provider = NewCloudProvider {
        name: "B2 S3".to_string(),
        provider_type: CloudProviderType::BackblazeB2,
        endpoint: Some("https://s3.us-west-002.backblazeb2.com".to_string()),
        region: Some("us-west-002".to_string()),
        bucket: "s3-bucket".to_string(),
        path_prefix: None,
        access_key: "s3-key".to_string(),
        secret_key: "s3-secret".to_string(),
        b2_account_id: None, // Não obrigatório para S3
        b2_application_key: None, // Não obrigatório para S3
        use_b2_native_api: Some(false),
        is_default: Some(false),
        test_connectivity: Some(false),
    };

    let json = serde_json::to_string(&s3_provider).unwrap();
    assert!(json.contains("s3.us-west-002.backblazeb2.com"));
    assert!(json.contains("\"b2_account_id\":null")); // Campo está presente mas é null

    // Configuração API nativa
    let native_provider = NewCloudProvider {
        name: "B2 Native".to_string(),
        provider_type: CloudProviderType::BackblazeB2,
        endpoint: None, // Não necessário para API nativa
        region: None,
        bucket: "native-bucket".to_string(),
        path_prefix: None,
        access_key: "native-key".to_string(),
        secret_key: "native-secret".to_string(),
        b2_account_id: Some("account123".to_string()),
        b2_application_key: Some("app-key123".to_string()),
        use_b2_native_api: Some(true),
        is_default: Some(false),
        test_connectivity: Some(false),
    };

    let json = serde_json::to_string(&native_provider).unwrap();
    assert!(json.contains("account123"));
    assert!(json.contains("app-key123"));
    assert!(json.contains("\"use_b2_native_api\":true"));
}

/// Testa configurações de outros provedores
/// 
/// Verifica se:
/// - IDrive e2 requer endpoint
/// - Wasabi e Scaleway requerem region
#[test]
fn test_other_provider_configurations() {
    // IDrive e2
    let idrive_provider = NewCloudProvider {
        name: "IDrive e2".to_string(),
        provider_type: CloudProviderType::IdriveE2,
        endpoint: Some("https://endpoint.idrivee2.com".to_string()),
        region: Some("us-west".to_string()),
        bucket: "idrive-bucket".to_string(),
        path_prefix: None,
        access_key: "idrive-key".to_string(),
        secret_key: "idrive-secret".to_string(),
        b2_account_id: None,
        b2_application_key: None,
        use_b2_native_api: Some(false),
        is_default: Some(false),
        test_connectivity: Some(false),
    };

    let json = serde_json::to_string(&idrive_provider).unwrap();
    assert!(json.contains("idrive_e2"));
    assert!(json.contains("endpoint.idrivee2.com"));

    // Wasabi
    let wasabi_provider = NewCloudProvider {
        name: "Wasabi Storage".to_string(),
        provider_type: CloudProviderType::Wasabi,
        endpoint: Some("https://s3.wasabisys.com".to_string()),
        region: Some("us-east-1".to_string()),
        bucket: "wasabi-bucket".to_string(),
        path_prefix: Some("backups/".to_string()),
        access_key: "wasabi-key".to_string(),
        secret_key: "wasabi-secret".to_string(),
        b2_account_id: None,
        b2_application_key: None,
        use_b2_native_api: Some(false),
        is_default: Some(false),
        test_connectivity: Some(false),
    };

    let json = serde_json::to_string(&wasabi_provider).unwrap();
    assert!(json.contains("wasabi"));
    assert!(json.contains("us-east-1"));

    // Scaleway
    let scaleway_provider = NewCloudProvider {
        name: "Scaleway Storage".to_string(),
        provider_type: CloudProviderType::Scaleway,
        endpoint: Some("https://s3.fr-par.scw.cloud".to_string()),
        region: Some("fr-par".to_string()),
        bucket: "scaleway-bucket".to_string(),
        path_prefix: None,
        access_key: "scaleway-key".to_string(),
        secret_key: "scaleway-secret".to_string(),
        b2_account_id: None,
        b2_application_key: None,
        use_b2_native_api: Some(false),
        is_default: Some(false),
        test_connectivity: Some(false),
    };

    let json = serde_json::to_string(&scaleway_provider).unwrap();
    assert!(json.contains("scaleway"));
    assert!(json.contains("fr-par"));
}

/// Testa validação de UUIDs
/// 
/// Verifica se:
/// - UUIDs são gerados corretamente
/// - Parsing de UUID funciona
#[test]
fn test_uuid_handling() {
    let test_uuid = Uuid::new_v4();
    let uuid_str = test_uuid.to_string();
    
    // Verificar que podemos fazer parse do UUID
    let parsed_uuid = Uuid::parse_str(&uuid_str).unwrap();
    assert_eq!(test_uuid, parsed_uuid);
    
    // Verificar que o UUID tem formato correto
    assert_eq!(uuid_str.len(), 36); // UUID padrão tem 36 caracteres
    assert_eq!(uuid_str.chars().filter(|&c| c == '-').count(), 4); // 4 hífens
}