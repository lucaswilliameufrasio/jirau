# ==============================================================================
# 🪵 jirau - Makefile de Automação & Ferramentas
# CLI de Scaffold Nativo Multiplataforma (Rust + Android + iOS + Desktop)
# ==============================================================================

export ANDROID_HOME ?= $(HOME)/Android/Sdk
export ANDROID_NDK_HOME ?= $(ANDROID_HOME)/ndk/27.2.12479018
export PATH := $(HOME)/.local/bin:$(ANDROID_HOME)/platform-tools:$(ANDROID_HOME)/cmdline-tools/latest/bin:$(PATH)
MISE           := $(shell if [ -f "$$HOME/.local/bin/mise" ]; then echo "$$HOME/.local/bin/mise"; elif command -v mise >/dev/null 2>&1; then echo "mise"; else echo ""; fi)
RUN            := $(if $(MISE),$(MISE) exec --,)
CARGO          ?= $(shell command -v cargo 2>/dev/null || echo "$$HOME/.cargo/bin/cargo")
RUSTUP         ?= $(shell command -v rustup 2>/dev/null || echo "$$HOME/.cargo/bin/rustup")
ADB            ?= $(shell command -v adb 2>/dev/null || echo "$$HOME/Android/Sdk/platform-tools/adb")

.PHONY: all help setup doctor devices test test-unit test-integration check clippy format format-check validate build clean

all: help

help:
	@echo "🪵 jirau - Comandos disponíveis:"
	@echo ""
	@echo "🛠️  Setup & Ambiente:"
	@echo "  make setup                  - Instala/verifica tooling (mise, Java 21, Rust targets, cargo-ndk)"
	@echo "  make doctor                 - Executa o diagnóstico completo de ambiente do jirau"
	@echo "  make devices                - Lista dispositivos e emuladores detectados (ADB / Simctl)"
	@echo ""
	@echo "🧪 Testes & Qualidade:"
	@echo "  make test                   - Executa todos os testes unitários e de integração"
	@echo "  make test-unit              - Roda apenas os testes unitários rápidos"
	@echo "  make test-integration       - Roda testes de integração de scaffolding e doctor"
	@echo "  make check                  - Executa cargo check e cargo clippy"
	@echo "  make clippy                 - Executa cargo clippy com -D warnings"
	@echo "  make format                 - Formata o código Rust (cargo fmt)"
	@echo "  make format-check           - Valida a formatação sem alterar arquivos"
	@echo "  make validate               - format-check + clippy + test"
	@echo ""
	@echo "🏗️  Build:"
	@echo "  make build                  - Compila o binário da CLI jirau em release"
	@echo "  make clean                  - Limpa a pasta target/ e caches"

setup:
	@echo "==> [1/3] Verificando ambiente Android e NDK..."
	@if [ -f scripts/setup-android-env.sh ]; then ./scripts/setup-android-env.sh; fi
	@echo "==> [2/3] Instalando targets Android para cargo-ndk..."
	@$(RUSTUP) target add aarch64-linux-android x86_64-linux-android armv7-linux-androideabi i686-linux-android 2>/dev/null || true
	@if [ "$$(uname -s)" = "Darwin" ]; then \
		echo "==> [3/3] Instalando targets iOS (Apple Silicon)..."; \
		$(RUSTUP) target add aarch64-apple-ios aarch64-apple-ios-sim 2>/dev/null || true; \
	fi
	@echo "Setup do jirau concluído!"

doctor:
	@$(CARGO) run -- doctor

devices:
	@echo "==> Dispositivos detectados:"
	@command -v adb >/dev/null 2>&1 && adb devices -l || echo "adb não encontrado no PATH"
	@if [ "$$(uname -s)" = "Darwin" ]; then \
		echo "--- iOS Simulators Booted ---"; \
		xcrun simctl list devices 2>/dev/null | grep -i booted || echo "Nenhum simulador iOS ativo"; \
	fi

test-unit:
	@$(CARGO) test --bin jirau

test-integration:
	@$(CARGO) test --test integration_test

test:
	@$(CARGO) test

check:
	@$(CARGO) check --workspace

clippy:
	@$(CARGO) clippy --all-targets -- -D warnings

format:
	@$(CARGO) fmt

format-check:
	@$(CARGO) fmt --check

validate: format-check clippy test

build:
	@$(CARGO) build --release

clean:
	@$(CARGO) clean
