# B2CLI API Design Philosophy

This document outlines the RESTful design principles used in the b2cli API.

## Resources, Not Actions

The API is designed around **Resources** (nouns) rather than **Actions** (verbs). The URL identifies the resource you want to interact with, and the HTTP Method (`GET`, `POST`, `DELETE`, etc.) defines what you want to do with that resource.

## Core Resources

### The `Backups` Resource

The primary resource in this API is the `Backup` (or `BackupJob`). It represents a configured backup task.

| Method | Route                   | Description                                          | Status Code |
| :----- | :---------------------- | :--------------------------------------------------- | :---------- |
| `POST` | `/backups`              | Creates a new backup configuration.                  | 201 Created |
| `GET`  | `/backups`              | Lists all active backup configurations.              | 200 OK      |
| `GET`  | `/backups/{id}`         | Gets the details of a specific backup configuration. | 200 OK / 404 Not Found |
| `DELETE` | `/backups/{id}`       | Soft deletes a backup configuration.                 | 204 No Content / 404 Not Found |
| `POST` | `/backups/{id}/run`     | Starts the execution of a specific backup.           | 200 OK / 404 Not Found |

### System Endpoints

| Method | Route         | Description                              | Status Code |
| :----- | :------------ | :--------------------------------------- | :---------- |
| `GET`  | `/health`     | Simple health check                      | 200 OK      |
| `GET`  | `/readiness`  | Detailed readiness check with DB status  | 200 OK / 503 Service Unavailable |

## Request & Response Formats

### Creating a Backup Job

```json
POST /backups
{
  "name": "Daily Documents Backup",
  "mappings": {
    "/home/user/Documents": [
      "/mnt/backup/daily",
      "s3://my-bucket/documents"
    ]
  }
}
```

Response:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "Daily Documents Backup",
  "mappings": {
    "/home/user/Documents": [
      "/mnt/backup/daily",
      "s3://my-bucket/documents"
    ]
  },
  "status": "PENDING",
  "created_at": "2025-08-01T10:00:00Z",
  "updated_at": "2025-08-01T10:00:00Z",
  "deleted_at": null
}
```

### Error Response Format

All errors follow a consistent format:

```json
{
  "message": "Detailed error message here"
}
```

## Design Principles

### 1. Soft Delete

Backups are never permanently deleted via the API. The `DELETE` operation sets:
- `deleted_at` timestamp
- `is_active = false`

This allows for:
- Audit trails
- Recovery of accidentally deleted jobs
- Historical analysis

### 2. Idempotency

- `DELETE` operations are idempotent - deleting an already deleted resource returns 404
- `POST /backups/{id}/run` can be called multiple times safely

### 3. Status Management

Backup jobs have the following statuses:
- `PENDING` - Initial state after creation
- `RUNNING` - Currently executing
- `COMPLETED` - Successfully finished
- `FAILED` - Execution failed

### 4. Filtering

The `GET /backups` endpoint automatically filters out soft-deleted jobs (`is_active = false`).

## Future Considerations

### Planned Endpoints

- `GET /backups/{id}/files` - List files backed up by a job
- `GET /backups/{id}/history` - Execution history of a job
- `PUT /backups/{id}` - Update backup configuration
- `POST /backups/{id}/restore` - Restore files from a backup

### Authentication

Future versions will include:
- API key authentication
- JWT tokens for user sessions
- Role-based access control (RBAC)
