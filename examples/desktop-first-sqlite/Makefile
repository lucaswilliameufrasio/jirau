# ==============================================================================
# 🪵 desktop-first-sqlite - Makefile de Automação & Ferramentas
# Gerado via jirau (CLI de scaffold nativo multiplataforma)
# ==============================================================================

CARGO          ?= $(shell command -v cargo 2>/dev/null || echo "$$HOME/.cargo/bin/cargo")

.PHONY: all help doctor test test-core check format clean build-desktop run-desktop

all: help

help:
	@echo "🪵 desktop-first-sqlite - Comandos disponíveis:"
	@echo ""
	@echo "  make doctor                 - Executa diagnóstico do ambiente de desenvolvimento"
	@echo "  make test                   - Executa todos os testes da workspace (Rust Core + integrações)"
	@echo "  make test-core              - Executa testes unitários do núcleo Rust puro no host"
	@echo "  make check                  - Executa cargo check e cargo clippy"
	@echo "  make format                 - Formata o código Rust (cargo fmt)"
	@echo "  make build-core             - Compila as crates Rust da workspace"
	@echo "  make build-desktop          - Compila o binário nativo Desktop em GPU via Rune"
	@echo "  make run-desktop            - Executa o app Desktop em desenvolvimento"
	@echo "  make clean                  - Limpa artefatos de compilação"

doctor:
	@command -v jirau >/dev/null 2>&1 && jirau doctor || $(CARGO) run -- doctor

test-core:
	@echo "==> Executando testes do núcleo Rust no host..."
	@$(CARGO) test --workspace

test: test-core

check:
	@echo "==> Executando cargo check e cargo clippy..."
	@$(CARGO) check --workspace
	@$(CARGO) clippy --all-targets -- -D warnings

format:
	@echo "==> Formatando código Rust..."
	@$(CARGO) fmt

build-core:
	@echo "==> Compilando crates Rust..."
	@$(CARGO) build --workspace

clean:
	@echo "==> Limpando artefatos de build..."
	@$(CARGO) clean

build-desktop:
	@$(CARGO) build --package desktop-first-sqlite-desktop

run-desktop:
	@$(CARGO) run --package desktop-first-sqlite-desktop
