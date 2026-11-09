# ==============================================================================
# 🪵 desktop-first-sqlite - Makefile de Automação & Ferramentas
# Gerado via jirau (CLI de scaffold nativo multiplataforma)
# ==============================================================================

export ANDROID_HOME ?= $(HOME)/Android/Sdk
export PATH := $(HOME)/.local/bin:$(ANDROID_HOME)/platform-tools:$(ANDROID_HOME)/cmdline-tools/latest/bin:$(PATH)
IOS_DEVICE     ?= $(shell xcrun simctl list devices 2>/dev/null | grep -i booted | grep -oE '[0-9A-F-]{36}' | head -1)
MISE           := $(shell if [ -f "$$HOME/.local/bin/mise" ]; then echo "$$HOME/.local/bin/mise"; elif command -v mise >/dev/null 2>&1; then echo "mise"; else echo ""; fi)
RUN            := $(if $(MISE),$(MISE) exec --,)
CARGO          ?= $(shell command -v cargo 2>/dev/null || echo "$$HOME/.cargo/bin/cargo")
RUSTUP         ?= $(shell command -v rustup 2>/dev/null || echo "$$HOME/.cargo/bin/rustup")
ADB            ?= $(shell command -v adb 2>/dev/null || echo "$$HOME/Android/Sdk/platform-tools/adb")

.PHONY: all help setup doctor devices test test-core check format clean

all: help

help:
	@echo "🪵 desktop-first-sqlite - Comandos disponíveis:"
	@echo ""
	@echo "  make setup                  - Instala/verifica dependências de ambiente (mise, Java, Rust targets)"
	@echo "  make doctor                 - Executa diagnóstico do ambiente de desenvolvimento"
	@echo "  make devices                - Lista emuladores e dispositivos móveis conectados"
	@echo "  make test                   - Executa todos os testes da workspace (Rust Core + integrações)"
	@echo "  make test-core              - Executa testes unitários do núcleo Rust puro no host"
	@echo "  make check                  - Executa cargo check e cargo clippy"
	@echo "  make format                 - Formata o código Rust (cargo fmt)"
	@echo "  make build-core             - Compila as crates Rust da workspace"
	@echo "  make build-desktop          - Compila o binário nativo Desktop em GPU via Rune"
	@echo "  make run-desktop            - Executa o app Desktop em desenvolvimento"
	@echo "  make clean                  - Limpa artefatos de compilação"

setup:
	@echo "==> Configurando ambiente do projeto..."
	@echo "  Nenhuma etapa adicional requerida para o ambiente."
	@echo "Setup concluído!"

doctor:
	@command -v jirau >/dev/null 2>&1 && jirau doctor || $(CARGO) run -- doctor

devices:
	@echo "==> Dispositivos detectados:"
	@command -v adb >/dev/null 2>&1 && adb devices -l || echo "adb não encontrado no PATH"
	@if [ "$$(uname -s)" = "Darwin" ]; then \
		echo "--- iOS Simulators Booted ---"; \
		xcrun simctl list devices 2>/dev/null | grep -i booted || echo "Nenhum simulador iOS ativo"; \
	fi

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
	@if [ -d android ]; then cd android && rm -rf .gradle build app/build; fi

build-desktop:
	@$(CARGO) build --package desktop-first-sqlite-desktop

run-desktop:
	@$(CARGO) run --package desktop-first-sqlite-desktop

