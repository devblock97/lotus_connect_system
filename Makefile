# Makefile for Lotus Connect System (Podman Edition)

# Resolve Podman binary or fallback to PATH
PODMAN ?= $(shell which podman 2>/dev/null || echo "/opt/podman/bin/podman")
COMPOSE := $(PODMAN) compose -f compose.yaml

.PHONY: help up down restart ps logs run test clean

help:
	@echo "Available commands:"
	@echo "  make up       - Start background services (Postgres, Redis, MinIO) via Podman"
	@echo "  make down     - Stop all background containers"
	@echo "  make restart  - Restart background services"
	@echo "  make ps       - View running container status"
	@echo "  make logs     - View logs of background containers"
	@echo "  make run      - Run the Lotus Connect Axum backend server"
	@echo "  make test     - Run all backend integration and unit tests"
	@echo "  make clean    - Stop services and remove container volumes"

up:
	@echo "Starting services with Podman..."
	$(COMPOSE) up -d
	@echo "Services started. Run 'make ps' to view status."

down:
	@echo "Stopping services..."
	$(COMPOSE) down

restart: down up

ps:
	$(COMPOSE) ps

logs:
	$(COMPOSE) logs -f

run:
	cargo run

test:
	cargo test

clean:
	$(COMPOSE) down -v
