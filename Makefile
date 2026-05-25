# Sandbox Platform — top-level Makefile
#
# Every cargo invocation uses SQLX_OFFLINE=true so builds never
# require a live Postgres. The sqlx offline cache lives in
# crates/plane/.sqlx/ and is committed to the repo.
#
# To regenerate the cache after schema changes:
#   make sqlx-prepare  (requires DATABASE_URL)

SHELL      := /bin/bash
CARGO      := SQLX_OFFLINE=true cargo
CARGO_OPTS := --manifest-path crates/Cargo.toml
CRATES     := crates

# Disable incremental compilation for test targets — parallel test binaries
# race on target/debug/incremental/*.part.bin files, causing sporadic
# "failed to create dependency graph" errors on macOS APFS.
CARGO_TEST := CARGO_INCREMENTAL=0 $(CARGO)

.DEFAULT_GOAL := help

# ─── Help ───────────────────────────────────────────────────────────

.PHONY: help
help: ## Show this help.
	@awk 'BEGIN {FS = ":.*##"; printf "Usage: make \033[36m<target>\033[0m\n"} \
		/^[a-zA-Z_0-9-]+:.*?##/ { printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2 } \
		/^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) }' $(MAKEFILE_LIST)

# ─── Dev (daily driver) ────────────────────────────────────────────

##@ Dev

.PHONY: build
build: ## Build the whole workspace.
	$(CARGO) build $(CARGO_OPTS) --workspace

.PHONY: check
check: ## Type-check without building (fast).
	$(CARGO) check $(CARGO_OPTS) --workspace

.PHONY: fmt
fmt: ## Format all code.
	$(CARGO) fmt $(CARGO_OPTS) --all

.PHONY: lint
lint: ## Run clippy with deny warnings.
	$(CARGO) clippy $(CARGO_OPTS) --workspace --all-targets --all-features -- -D warnings

.PHONY: test
test: ## Run workspace tests (excludes Docker-dependent plane-tests).
	$(CARGO_TEST) test $(CARGO_OPTS) --workspace --all-features --exclude plane-tests

.PHONY: test-unit
test-unit: ## Run only unit tests (no integration, no Docker).
	$(CARGO_TEST) test $(CARGO_OPTS) --workspace --all-features --lib

.PHONY: docker-check
docker-check: ## Verify Docker is running.
	@docker info >/dev/null 2>&1 || (echo "ERROR: Docker is not running." && exit 1)

.PHONY: test-docker
test-docker: docker-check test-image ## Run Docker-dependent integration tests.
	$(CARGO_TEST) test $(CARGO_OPTS) -p plane-tests --all-features

# ─── CI ─────────────────────────────────────────────────────────────

##@ CI

.PHONY: ci
ci: fmt-check lint test ## CI pipeline (no Docker).

.PHONY: ci-full
ci-full: fmt-check lint test test-docker ## CI pipeline with Docker integration tests.

.PHONY: fmt-check
fmt-check: ## Check formatting without modifying.
	$(CARGO) fmt $(CARGO_OPTS) --all -- --check

# ─── Docker images ──────────────────────────────────────────────────

##@ Docker images

.PHONY: test-image
test-image: ## Build sandbox-runtime:test-latest (Alpine HTTP stub).
	@docker build -q -t sandbox-runtime:test-latest runtime-images/test/

.PHONY: image-base
image-base: ## Build sandbox-runtime:base-latest.
	@bash runtime-images/build.sh base latest

.PHONY: image-node24
image-node24: ## Build sandbox-runtime:node24-latest.
	@bash runtime-images/build.sh node24 latest

.PHONY: image-python313
image-python313: ## Build sandbox-runtime:python313-latest.
	@bash runtime-images/build.sh python313 latest

.PHONY: image-agent
image-agent: ## Build sandbox-runtime:agent-latest.
	@bash runtime-images/build.sh agent latest

.PHONY: image-desktop
image-desktop: ## Build sandbox-runtime:desktop-latest.
	@bash runtime-images/build.sh desktop latest

.PHONY: image-plane
image-plane: docker-check ## Build chamuka-plane:latest for the local cluster.
	docker build -t chamuka-plane:latest -f crates/plane/docker/Dockerfile crates

.PHONY: image-ysweet
image-ysweet: ## Build sandbox-runtime:ysweet-latest.
	@bash runtime-images/build.sh ysweet latest

.PHONY: fleet-images
fleet-images: image-plane image-node24 ## Build images required by the canonical Vagrant fleet.

.PHONY: images
images: test-image image-base image-node24 image-python313 image-agent image-desktop ## Build all runtime images.

# ─── Contracts / schemas ────────────────────────────────────────────

##@ Contracts

.PHONY: schemas
schemas: ## Regenerate JSON schemas from Rust types.
	$(CARGO) run $(CARGO_OPTS) --bin generate-schemas

.PHONY: schemas-check
schemas-check: schemas ## Fail if schemas are stale.
	@git diff --quiet contracts/schemas/ \
		|| (echo "ERROR: contracts/schemas/ is stale. Run 'make schemas' and commit." && exit 1)

# ─── sqlx ───────────────────────────────────────────────────────────

##@ Database

.PHONY: sqlx-prepare
sqlx-prepare: ## Regenerate sqlx offline cache (requires DATABASE_URL).
	cd $(CRATES)/plane && cargo sqlx prepare

# ─── Dev fleet ──────────────────────────────────────────────────────

##@ Dev fleet

.PHONY: fleet-up
fleet-up: ## Start canonical Vagrant dev fleet (host services + VM drones).
	@$(MAKE) -C dev-fleet up

.PHONY: fleet-down
fleet-down: ## Stop Vagrant drones (host services keep running).
	@$(MAKE) -C dev-fleet down

.PHONY: fleet-nuke
fleet-nuke: ## Destroy Vagrant fleet (VMs + host services + volumes).
	@$(MAKE) -C dev-fleet nuke

.PHONY: fleet-status
fleet-status: ## Show canonical Vagrant fleet status.
	@$(MAKE) -C dev-fleet status

.PHONY: smoke
smoke: ## End-to-end smoke test against canonical Vagrant fleet.
	@$(MAKE) -C dev-fleet smoke

.PHONY: fleet-up-docker fleet-down-docker fleet-status-docker smoke-docker
fleet-up-docker: ## Start Docker-only fallback fleet.
	@$(MAKE) -C dev-fleet docker-up

fleet-down-docker: ## Stop Docker-only fallback fleet.
	@$(MAKE) -C dev-fleet docker-down

fleet-status-docker: ## Show Docker-only fallback fleet status.
	@$(MAKE) -C dev-fleet docker-status

smoke-docker: ## Run smoke test against Docker-only fallback fleet.
	@$(MAKE) -C dev-fleet smoke-docker

# ─── Ops ────────────────────────────────────────────────────────────

##@ Ops

.PHONY: status
status: ## Show project phase, next step, and recent phase tags.
	@./scripts/status.sh

.PHONY: hooks
hooks: ## Configure git hooks.
	git config core.hooksPath .githooks
	@echo "Git hooks configured: .githooks/"

.PHONY: clean
clean: ## Remove build artifacts.
	$(CARGO) clean $(CARGO_OPTS)

# ─── Console ────────────────────────────────────────────────────────

##@ Console

.PHONY: console-install console-dev console-dev-next console-dev-relay console-build console-test console-lint

console-install: ## Install console npm dependencies.
	cd console && npm install

console-dev: ## Start console dev server (Next.js :3000 + ws-relay :3101).
	cd console && npm run dev

console-dev-next: ## Start Next.js dev server only (:3000).
	cd console && npm run dev:next

console-dev-relay: ## Start ws-relay dev server only (:3101).
	cd console && npm run dev:relay

console-build: ## Build console for production.
	cd console && npm run build

console-test: ## Run console test suite.
	cd console && npm run test

console-lint: ## Lint console sources.
	cd console && npm run lint
