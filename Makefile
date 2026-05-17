# ── Config ────────────────────────────────────────────────────────────────────
PLATFORM_URL = postgres://emakao:password@localhost:5432/emakao_platform
AGENCY_URL   = postgres://emakao:password@localhost:5432/emakao_agency
PREPARE_URL  = $(PLATFORM_URL)?options=-c%20search_path%3Ddev_agency,public

# ── Shortcuts ─────────────────────────────────────────────────────────────────

.PHONY: help
help:
	@echo ""
	@echo "  make up          Start Postgres containers"
	@echo "  make down        Stop containers"
	@echo "  make migrate     Run all migrations (platform + agency)"
	@echo "  make prepare     Generate .sqlx offline cache"
	@echo "  make build       Build the main crate (no DB needed)"
	@echo "  make dev         Run with hot-reload (requires cargo-watch)"
	@echo "  make setup       Fresh setup: up + migrate + prepare + build"
	@echo "  make reset       Wipe volumes, then full setup"
	@echo ""

# ── Docker ────────────────────────────────────────────────────────────────────

.PHONY: up
up:
	docker-compose up -d
	@echo "Waiting for Postgres to be ready..."
	@until docker-compose exec -T postgres pg_isready -U emakao > /dev/null 2>&1; do sleep 1; done
	@echo "✓ Postgres is ready"

.PHONY: down
down:
	docker-compose down

.PHONY: reset
reset:
	docker-compose down -v
	$(MAKE) setup

# ── Migrations ────────────────────────────────────────────────────────────────

.PHONY: migrate
migrate:
	PLATFORM_DATABASE_URL="$(PLATFORM_URL)" \
	AGENCY_DATABASE_URL="$(AGENCY_URL)" \
	cargo run -p migrate

# ── sqlx offline cache ────────────────────────────────────────────────────────

.PHONY: prepare
prepare:
	DATABASE_URL="$(PREPARE_URL)" \
	cargo sqlx prepare --workspace

# ── Build ─────────────────────────────────────────────────────────────────────

.PHONY: build
build:
	cargo build

.PHONY: build-release
build-release:
	cargo build --release

# ── Development ───────────────────────────────────────────────────────────────

# requires cargo-watch: cargo install cargo-watch
.PHONY: dev
dev:
	PLATFORM_DATABASE_URL="$(PLATFORM_URL)" \
	AGENCY_DATABASE_URL="$(AGENCY_URL)" \
	cargo watch -x run

# ── Full setup (new machine / fresh clone) ────────────────────────────────────

.PHONY: setup
setup: up migrate prepare build
	@echo ""
	@echo "✓ Setup complete — ready to develop"