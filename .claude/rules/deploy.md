---
paths:
  - "deploy/**"
  - "Dockerfile"
description: "Rules for deployment infrastructure"
---

# Deployment Rules

## Single binary architecture (ADR-001)

All routes served from one `mez-api` binary. No Python microservices. No separate corridor-node, watcher, entity-registry, or license-registry containers.

## Secret handling

All compose/deploy files use `${VAR:?must be set}`. No default credentials. `deploy-zone.sh` generates random secrets at deploy time.

## Docker

Multi-stage Dockerfile: `rust:1.77-bookworm` builder → `debian:bookworm-slim` runtime. Non-root user (`mez:mez`). Copies `modules/`, `schemas/`, `jurisdictions/` into image.

## Terraform (AWS)

`main.tf`: EKS + RDS (Multi-AZ) + KMS + ElastiCache Redis + S3 + VPC (3 AZs). `kubernetes.tf`: single mez-api deployment with IRSA, ALB ingress, TLS.

## Zone profiles

6 profiles: minimal-mvp, digital-financial-center, sovereign-govos, sovereign-govos-pk, sovereign-govos-ae, trade-hub. 210 jurisdiction definitions in `jurisdictions/`.
