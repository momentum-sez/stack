.PHONY: up down logs status full-stack full-stack-down kernel kernel-down corridor corridor-down validate clean

COMPOSE_STANDALONE := docker compose -f deploy/docker-compose.yaml --env-file .env
COMPOSE_FULL       := docker compose -f deploy/docker-compose.full-stack.yaml --env-file .env
COMPOSE_KERNEL     := docker compose -f deploy/docker-compose.kernel.yaml --env-file .env
COMPOSE_CORRIDOR   := docker compose -f deploy/docker-compose.two-zone.yaml --env-file .env

# ── Standalone ────────────────────────────────────────────────────────────────

up:
	$(COMPOSE_STANDALONE) up -d

down:
	$(COMPOSE_STANDALONE) down

# ── Full Stack ────────────────────────────────────────────────────────────────

full-stack:
	$(COMPOSE_FULL) up -d

full-stack-down:
	$(COMPOSE_FULL) down

# ── Kernel Topology ───────────────────────────────────────────────────────────

kernel:
	$(COMPOSE_KERNEL) up -d

kernel-down:
	$(COMPOSE_KERNEL) down

# ── Two-Zone Corridor ─────────────────────────────────────────────────────────

corridor:
	$(COMPOSE_CORRIDOR) up -d

corridor-down:
	$(COMPOSE_CORRIDOR) down

# ── Operations ────────────────────────────────────────────────────────────────

logs:
	$(COMPOSE_STANDALONE) logs -f mez-api

status:
	@curl -sf http://localhost:$${PORT:-8080}/health/liveness 2>/dev/null \
		&& echo "UP" || echo "DOWN"

clean:
	$(COMPOSE_STANDALONE) down -v
	$(COMPOSE_FULL) down -v 2>/dev/null || true
	$(COMPOSE_KERNEL) down -v 2>/dev/null || true
	$(COMPOSE_CORRIDOR) down -v 2>/dev/null || true

# ── Validate ──────────────────────────────────────────────────────────────────

validate: validate-zone validate-operations

validate-zone:
	@echo "Checking zone.yaml..."
	@test -f zone.yaml || { echo "ERROR: zone.yaml not found"; exit 1; }
	@if command -v check-jsonschema >/dev/null 2>&1; then \
		check-jsonschema --schemafile schemas/zone.schema.json zone.yaml; \
	else \
		echo "SKIP: install check-jsonschema for schema validation"; \
	fi

validate-operations:
	@echo "Checking operations..."
	@find operations -name "*.yaml" -o -name "*.yml" | while read f; do \
		echo "  $$f"; \
		if command -v check-jsonschema >/dev/null 2>&1; then \
			check-jsonschema --schemafile schemas/operation.schema.json "$$f" || exit 1; \
		fi; \
	done
	@echo "OK"
