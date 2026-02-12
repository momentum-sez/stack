//! # OpenAPI Sanity Tests
//!
//! Tests that OpenAPI-like structures canonicalize correctly, ensuring that
//! path definitions, schema references, response structures, and version
//! prefixes produce deterministic digests suitable for content-addressed
//! storage of API specifications.

use msez_core::{sha256_digest, CanonicalBytes};
use serde_json::json;

// ---------------------------------------------------------------------------
// 1. OpenAPI path structure canonicalizes correctly
// ---------------------------------------------------------------------------

#[test]
fn openapi_path_structure_canonical() {
    let path_def = json!({
        "/v1/entities": {
            "post": {
                "summary": "Create entity",
                "operationId": "createEntity",
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {"$ref": "#/components/schemas/CreateEntityRequest"}
                        }
                    }
                },
                "responses": {
                    "201": {"description": "Entity created"},
                    "400": {"description": "Bad request"},
                    "401": {"description": "Unauthorized"}
                }
            },
            "get": {
                "summary": "List entities",
                "operationId": "listEntities"
            }
        }
    });

    let d1 = sha256_digest(&CanonicalBytes::new(&path_def).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&path_def).unwrap());
    assert_eq!(d1, d2, "OpenAPI path structure digest must be deterministic");

    // Canonical bytes should be valid UTF-8
    let cb = CanonicalBytes::new(&path_def).unwrap();
    assert!(std::str::from_utf8(cb.as_bytes()).is_ok());
}

#[test]
fn openapi_path_key_order_irrelevant() {
    // "get" before "post" vs "post" before "get" should produce same digest
    let v1 = json!({
        "/v1/entities": {
            "post": {"summary": "Create"},
            "get": {"summary": "List"}
        }
    });
    let v2 = json!({
        "/v1/entities": {
            "get": {"summary": "List"},
            "post": {"summary": "Create"}
        }
    });

    let d1 = sha256_digest(&CanonicalBytes::new(&v1).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&v2).unwrap());
    assert_eq!(d1, d2, "key order in path definition must not affect digest");
}

// ---------------------------------------------------------------------------
// 2. OpenAPI schema $ref canonicalizes correctly
// ---------------------------------------------------------------------------

#[test]
fn openapi_schema_ref_canonical() {
    let schema_def = json!({
        "components": {
            "schemas": {
                "Entity": {
                    "type": "object",
                    "properties": {
                        "entity_id": {"type": "string", "format": "uuid"},
                        "name": {"type": "string", "minLength": 1},
                        "jurisdiction_id": {"$ref": "#/components/schemas/JurisdictionId"},
                        "status": {"$ref": "#/components/schemas/EntityLifecycleState"}
                    },
                    "required": ["entity_id", "name", "jurisdiction_id"]
                },
                "JurisdictionId": {
                    "type": "string",
                    "pattern": "^[A-Z]{2}-[A-Z]{4}$"
                },
                "EntityLifecycleState": {
                    "type": "string",
                    "enum": ["APPLIED", "ACTIVE", "SUSPENDED", "DISSOLVING", "DISSOLVED", "REJECTED"]
                }
            }
        }
    });

    let d1 = sha256_digest(&CanonicalBytes::new(&schema_def).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&schema_def).unwrap());
    assert_eq!(d1, d2, "OpenAPI schema definition digest must be deterministic");

    // Verify the $ref strings are preserved in canonical output
    let cb = CanonicalBytes::new(&schema_def).unwrap();
    let s = std::str::from_utf8(cb.as_bytes()).unwrap();
    assert!(s.contains("#/components/schemas/JurisdictionId"));
}

// ---------------------------------------------------------------------------
// 3. OpenAPI response structure canonicalizes correctly
// ---------------------------------------------------------------------------

#[test]
fn openapi_response_structure_canonical() {
    let response = json!({
        "responses": {
            "200": {
                "description": "Successful operation",
                "content": {
                    "application/json": {
                        "schema": {
                            "type": "object",
                            "properties": {
                                "data": {"type": "array", "items": {"$ref": "#/components/schemas/Entity"}},
                                "pagination": {
                                    "type": "object",
                                    "properties": {
                                        "total": {"type": "integer"},
                                        "page": {"type": "integer"},
                                        "per_page": {"type": "integer"}
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "400": {
                "description": "Bad request",
                "content": {
                    "application/json": {
                        "schema": {
                            "$ref": "#/components/schemas/ErrorResponse"
                        }
                    }
                }
            },
            "500": {
                "description": "Internal server error"
            }
        }
    });

    let d1 = sha256_digest(&CanonicalBytes::new(&response).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&response).unwrap());
    assert_eq!(d1, d2, "OpenAPI response structure digest must be deterministic");
}

// ---------------------------------------------------------------------------
// 4. API version prefix /v1/
// ---------------------------------------------------------------------------

#[test]
fn api_version_prefix_v1() {
    // Verify that v1-prefixed paths are distinct from non-prefixed
    let v1_path = json!({"path": "/v1/entities"});
    let v2_path = json!({"path": "/v2/entities"});
    let no_version = json!({"path": "/entities"});

    let d_v1 = sha256_digest(&CanonicalBytes::new(&v1_path).unwrap());
    let d_v2 = sha256_digest(&CanonicalBytes::new(&v2_path).unwrap());
    let d_nv = sha256_digest(&CanonicalBytes::new(&no_version).unwrap());

    assert_ne!(d_v1, d_v2, "v1 and v2 paths must produce different digests");
    assert_ne!(d_v1, d_nv, "v1 and unversioned paths must produce different digests");
    assert_ne!(d_v2, d_nv, "v2 and unversioned paths must produce different digests");
}

// ---------------------------------------------------------------------------
// 5. Full OpenAPI-like spec canonicalizes
// ---------------------------------------------------------------------------

#[test]
fn full_openapi_like_spec_canonical() {
    let spec = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "MSEZ Entities API",
            "version": "1.0.0",
            "description": "Entity lifecycle management for Special Economic Zones"
        },
        "servers": [
            {"url": "https://api.momentum-sez.org/v1"}
        ],
        "paths": {
            "/v1/entities": {
                "post": {"operationId": "createEntity"},
                "get": {"operationId": "listEntities"}
            },
            "/v1/entities/{entity_id}": {
                "get": {"operationId": "getEntity"}
            }
        },
        "security": [
            {"bearerAuth": []}
        ]
    });

    let d1 = sha256_digest(&CanonicalBytes::new(&spec).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&spec).unwrap());
    assert_eq!(d1, d2);
    assert_eq!(d1.to_hex().len(), 64);
}
