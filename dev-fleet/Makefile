# Sandbox Platform — local dev fleet
#
# Bring up host services (Postgres, Plane controller, proxy, registry)
# in containers on the host, plus N VirtualBox VMs that each run a
# Plane drone connected to the controller.
#
# All Plane API endpoints used here verified against
# github.com/jamsocket/plane source.

SHELL := /bin/bash
.DEFAULT_GOAL := help

DRONE_COUNT     ?= 3
DRONE_VCPUS     ?= 2
DRONE_MEM_MB    ?= 4096
CONTROLLER_PORT ?= 8080
PROXY_PORT      ?= 9090
HOST_ONLY_IP    ?= 192.168.56.1
CLUSTER_NAME    ?= $(HOST_ONLY_IP):$(PROXY_PORT)
PLANE_IMAGE     ?= chamuka-plane:latest
RUNTIME_IMAGE   ?= sandbox-runtime:node24-latest
REGISTRY        ?= localhost:5000
# Images as seen by drone VMs (local registry on host-only IP).
VAGRANT_PLANE_IMAGE   ?= $(HOST_ONLY_IP):5000/$(PLANE_IMAGE)
VAGRANT_RUNTIME_IMAGE ?= $(HOST_ONLY_IP):5000/$(RUNTIME_IMAGE)

export DRONE_COUNT DRONE_VCPUS DRONE_MEM_MB CONTROLLER_PORT PROXY_PORT \
       HOST_ONLY_IP CLUSTER_NAME PLANE_IMAGE RUNTIME_IMAGE REGISTRY \
       VAGRANT_PLANE_IMAGE VAGRANT_RUNTIME_IMAGE

# Cluster name URL-encoded (colon → %3A) for path segments.
CLUSTER_ENC := $(subst :,%3A,$(CLUSTER_NAME))

##@ General

.PHONY: help
help: ## Show this help.
	@awk 'BEGIN {FS = ":.*##"; printf "Usage: make \033[36m<target>\033[0m\n"} \
		/^[a-zA-Z_0-9-]+:.*?##/ { printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2 } \
		/^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) }' $(MAKEFILE_LIST)

##@ Lifecycle

.PHONY: ensure-images
ensure-images: ## Verify fleet images exist locally.
	@if ! docker image inspect $(PLANE_IMAGE) >/dev/null 2>&1; then \
		echo "ERROR: $(PLANE_IMAGE) not found. Run 'make fleet-images' from repo root."; \
		exit 1; \
	fi
	@if ! docker image inspect $(RUNTIME_IMAGE) >/dev/null 2>&1; then \
		echo "ERROR: $(RUNTIME_IMAGE) not found. Run 'make fleet-images' from repo root."; \
		exit 1; \
	fi

.PHONY: check
check: ## Verify prerequisites for canonical Vagrant fleet.
	@bash scripts/check-prereqs.sh

.PHONY: ensure-network
ensure-network: ## Ensure VirtualBox host-only network is defined.
	@bash scripts/ensure-hostonlynet.sh

.PHONY: up
up: ensure-images check ensure-network host-up registry-push drones-up ## Full cold-start: host services + registry + drones.
	@echo ""
	@echo "Fleet is up."
	@echo "Controller:  http://$(HOST_ONLY_IP):$(CONTROLLER_PORT)"
	@echo "Proxy:       http://$(HOST_ONLY_IP):$(PROXY_PORT)"
	@echo "Cluster:     $(CLUSTER_NAME)"
	@echo ""
	@echo "Run 'make smoke' to verify end-to-end."

.PHONY: down
down: drones-down ## Stop drones. Host services keep running.

.PHONY: nuke
nuke: drones-destroy host-down ## Destroy VMs and stop host services.
	@docker volume rm -f sandbox-platform-dev_pgdata 2>/dev/null || true
	@docker volume rm -f sandbox-platform-dev_registry 2>/dev/null || true
	@echo "All state nuked."

.PHONY: status
status: ## Show what's running (canonical Vagrant fleet).
	@echo "=== Host services ==="
	@cd host && docker compose ps 2>/dev/null || echo "(host services not running)"
	@echo ""
	@echo "=== Drones (Vagrant) ==="
	@vagrant status 2>/dev/null || echo "(no drones managed)"

##@ Docker fallback

.PHONY: docker-check
docker-check: ## Verify Docker is running.
	@docker info >/dev/null 2>&1 || (echo "ERROR: Docker is not running." && exit 1)

.PHONY: docker-conflict-check
docker-conflict-check: ## Refuse Docker fallback if Vagrant fleet is active.
	@bash scripts/check-docker-prereqs.sh

.PHONY: docker-up
docker-up: docker-check docker-conflict-check ensure-images ## Start Docker-only fallback fleet.
	@echo "Starting Docker-only fallback fleet..."
	@docker compose up -d
	@echo ""
	@echo "Docker fallback fleet is up."
	@echo "Controller:  http://localhost:$(CONTROLLER_PORT)"
	@echo "Proxy:       http://localhost:$(PROXY_PORT)"
	@echo "Cluster:     localhost:$(PROXY_PORT)"
	@echo ""
	@echo "Run 'make smoke-docker' to verify end-to-end."

.PHONY: docker-down
docker-down: ## Stop Docker-only fallback fleet.
	@docker compose down

.PHONY: docker-status
docker-status: ## Show Docker-only fallback fleet status.
	@docker compose ps 2>/dev/null || echo "(Docker fallback fleet not running)"

##@ Host (control plane)

.PHONY: host-up
host-up: ## Start Postgres, controller, proxy, registry.
	@echo "Starting host services..."
	@cd host && docker compose up -d
	@echo "Waiting for controller /pub/health..."
	@for i in $$(seq 1 60); do \
		if curl -sf http://127.0.0.1:$(CONTROLLER_PORT)/pub/health >/dev/null 2>&1; then \
			echo "Controller is healthy."; \
			exit 0; \
		fi; \
		sleep 1; \
	done; \
	echo "Controller not healthy after 60s. Run 'make logs' for details."; \
	exit 1

.PHONY: host-down
host-down: ## Stop host services (preserves volumes).
	@cd host && docker compose down

.PHONY: logs
logs: ## Tail controller logs.
	@cd host && docker compose logs -f controller

.PHONY: logs-all
logs-all: ## Tail all host service logs.
	@cd host && docker compose logs -f

##@ Registry

.PHONY: registry-push
registry-push: ## Tag and push required images to local registry.
	@echo "Pushing images to local registry ($(REGISTRY))..."
	@for img in $(PLANE_IMAGE) $(RUNTIME_IMAGE); do \
		if docker image inspect "$$img" >/dev/null 2>&1; then \
			docker tag "$$img" "$(REGISTRY)/$$img" && \
			docker push "$(REGISTRY)/$$img" && \
			echo "  pushed $$img → $(REGISTRY)/$$img"; \
		else \
			echo "  SKIP: $$img not found locally"; \
		fi; \
	done

##@ Drones (VMs)

.PHONY: drones-up
drones-up: ## Start all drone VMs.
	@vagrant up --parallel

.PHONY: drones-down
drones-down: ## Halt all drone VMs (preserves disk).
	@vagrant halt

.PHONY: drones-destroy
drones-destroy: ## Destroy all drone VMs.
	@vagrant destroy -f

.PHONY: drones-reload
drones-reload: ## Reload all drones (re-run provisioning).
	@vagrant reload --provision

ssh-%: ## ssh into drone-N, e.g. make ssh-1.
	@vagrant ssh drone-$*

##@ Verification

.PHONY: smoke
smoke: ## End-to-end test against canonical Vagrant fleet.
	@echo "=== Smoke test (Vagrant fleet) ==="
	@echo "  controller: 127.0.0.1:$(CONTROLLER_PORT)"
	@echo "  proxy:      $(HOST_ONLY_IP):$(PROXY_PORT)"
	@echo "  cluster:    $(CLUSTER_NAME)"
	@echo "  image:      $(HOST_ONLY_IP):5000/$(RUNTIME_IMAGE)"
	@echo ""
	@CONTROLLER=127.0.0.1:$(CONTROLLER_PORT) PROXY=$(HOST_ONLY_IP):$(PROXY_PORT) CLUSTER=$(CLUSTER_NAME) IMAGE=$(HOST_ONLY_IP):5000/$(RUNTIME_IMAGE) bash scripts/smoke-test.sh

.PHONY: smoke-docker
smoke-docker: ## End-to-end test against Docker-only fallback fleet.
	@echo "=== Smoke test (Docker fallback fleet) ==="
	@echo "  controller: localhost:$(CONTROLLER_PORT)"
	@echo "  proxy:      localhost:$(PROXY_PORT)"
	@echo "  cluster:    localhost:$(PROXY_PORT)"
	@echo "  image:      $(RUNTIME_IMAGE)"
	@echo ""
	@CONTROLLER=localhost:$(CONTROLLER_PORT) PROXY=localhost:$(PROXY_PORT) CLUSTER=localhost:$(PROXY_PORT) IMAGE=$(RUNTIME_IMAGE) bash scripts/smoke-test.sh

.PHONY: cluster-state
cluster-state: ## Show drones and proxies registered with the controller.
	@curl -sf "http://127.0.0.1:$(CONTROLLER_PORT)/ctrl/c/$(CLUSTER_ENC)/state" \
		| jq . 2>/dev/null \
		|| curl -sf "http://127.0.0.1:$(CONTROLLER_PORT)/ctrl/c/$(CLUSTER_ENC)/state"

##@ Convenience

.PHONY: env
env: ## Print effective config.
	@echo "DRONE_COUNT     = $(DRONE_COUNT)"
	@echo "DRONE_VCPUS     = $(DRONE_VCPUS)"
	@echo "DRONE_MEM_MB    = $(DRONE_MEM_MB)"
	@echo "CONTROLLER_PORT = $(CONTROLLER_PORT)"
	@echo "PROXY_PORT      = $(PROXY_PORT)"
	@echo "HOST_ONLY_IP    = $(HOST_ONLY_IP)"
	@echo "CLUSTER_NAME    = $(CLUSTER_NAME)"
	@echo "PLANE_IMAGE     = $(PLANE_IMAGE)"
	@echo "RUNTIME_IMAGE   = $(RUNTIME_IMAGE)"
	@echo "REGISTRY        = $(REGISTRY)"
