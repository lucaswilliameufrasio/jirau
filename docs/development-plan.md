# Plano — CLI de scaffold nativo (Rust + Android + iOS)

Nov 7, 2026 · @Lucas Eufrasio

## Objetivo e filosofia

Uma CLI em Rust que gera e opera projetos com a estrutura do satchel: um núcleo em Rust, um app Android em Kotlin com Compose e um app iOS em Swift com SwiftUI. O objetivo é ter a experiência do `flutter create` + `flutter doctor` + `flutter run` sem abrir mão da UI nativa.

O que a diferencia do Crux, do mobiler e do jffi:

- **O núcleo só tem lógica.** Ele não é dono da UI nem impõe uma arquitetura de mensagens e efeitos.
- **A UI é nativa e sem framework.** No Android: Navigation Compose, `ViewModel` + `StateFlow` e chamadas diretas ao núcleo. No iOS: `NavigationStack` e `@Observable`. É o mesmo princípio do Flutter com rota nativa e provider.
- **A ponte é UniFFI puro.** Nada de JNI escrito à mão nem camada própria entre o UniFFI e o app.
- **O projeto gerado não depende da CLI.** Depois do `new`, ele compila só com `cargo`, Gradle e Xcode. A CLI é um atalho, não um runtime.

A CLI vem primeiro e a TUI depois, só se fizer falta. Um `doctor` com saída colorida resolve a maior parte do caso.

O template sai do satchel, que já tem `crates/`, `android/` e `TESTING_STANDARD.md`. O primeiro projeto gerado por ela é o app do curupira.

## Nome: jirau

A CLI se chama **jirau**, e o arquivo de configuração do projeto gerado é o `jirau.toml`.

Nas casas ribeirinhas da Amazônia, o jirau é um estrado de madeira suspenso, feito para apoiar coisas. Na prática, é um andaime. Isso descreve o que a ferramenta faz: monta a estrutura em que o app se apoia e sai do caminho, porque o projeto gerado não depende dela.

O nome segue a mesma linha de curupira, tucupi, acari e sabiá, com termos regionais ligados à utilidade do projeto. Ele também é curto e fácil de digitar: `jirau new`, `jirau doctor`, `jirau dev`.

As outras opções consideradas foram tipiti (o cesto que dá forma à mandioca e de onde sai o tucupi), maloca (a casa comunal, vários apps sob um núcleo só) e puxirum (o mutirão). O jirau ficou por ser o único que significa estrutura de apoio.

## Estrutura gerada

O `new` gera um workspace Cargo na raiz, com os apps nativos como pastas irmãs. É a mesma forma do satchel, só que com o `-ffi` em UniFFI e o iOS já incluído.

```text
meu-app/
├── Cargo.toml               # workspace
├── crates/
│   ├── meu-app-core/        # lógica pura, sem FFI, testada com cargo test
│   └── meu-app-ffi/         # UniFFI: exporta o core para Kotlin e Swift
├── android/
│   ├── settings.gradle.kts
│   ├── gradle.properties    # já com caches ligados (ver Build rápido)
│   ├── gradle/libs.versions.toml
│   └── app/                 # Compose, Navigation, ViewModel
├── ios/
│   ├── project.yml          # XcodeGen; o .xcodeproj não é versionado
│   └── App/                 # SwiftUI, NavigationStack, @Observable
├── TESTING_STANDARD.md
└── jirau.toml               # nome, bundle id, ABIs de dev, versão do template
```

Decisões do template:

- **`core` e `ffi` separados.** O `core` não conhece UniFFI. Os testes rodam rápido no host, e o FFI só adapta tipos.
- **Bindings gerados ficam fora do Git.** O Kotlin vai para `android/app/build/generated/uniffi` e o Swift para `ios/Generated/`. Os dois são regerados pelo `bindings`.
- **Bibliotecas nativas.** No Android, `.so` por ABI em `jniLibs/`. No iOS, um `XCFramework` com device e simulador.
- **XcodeGen em vez de Tuist.** É um YAML só, sem DSL em Swift. Ele evita conflito de `.pbxproj` e deixa o iOS ser gerado em Linux, mesmo que compilar ainda exija o Mac.
- **Versões centralizadas** no `libs.versions.toml` do Android e no `Cargo.toml` do workspace.

## Comandos

São cinco comandos, cada um equivalente a um passo do fluxo do Flutter.

| Comando | O que faz | Equivalente no Flutter |
| --- | --- | --- |
| `new <nome>` | Gera o projeto a partir do template, com nome, pacote, bundle id e plataformas (`--android`, `--ios`). | `flutter create` |
| `doctor` | Confere rustup e os targets (`aarch64-linux-android`, `x86_64-linux-android`, `aarch64-apple-ios`, `aarch64-apple-ios-sim`), NDK, `cargo-ndk`, JDK, Android SDK, emulador, Xcode, XcodeGen e simuladores. Mostra o comando que corrige cada falta. | `flutter doctor` |
| `dev [android\|ios]` | Observa `crates/`, recompila o core só para o ABI do aparelho ativo, regera os bindings e reinstala. A UI fica com os previews e o Live Edit do IDE. | `flutter run` |
| `bindings` | Regera Kotlin e Swift a partir do `-ffi`. | `build_runner` (só a parte boa) |
| `build [android\|ios] --release` | Gera AAB/APK com os três ABIs e o XCFramework com device e simulador. | `flutter build` |

Detalhes:

- **O `dev` descobre o ABI sozinho.** Ele usa `adb shell getprop ro.product.cpu.abi` no Android e o simulador ou aparelho escolhido no iOS.
- **O `doctor` tem `--json`** para usar no CI.
- **Depois:** `upgrade`, que aplica uma versão nova do template com merge de três vias, como o mobiler faz. A versão do template fica no arquivo de configuração.

## Build rápido

O template já sai configurado para o ciclo curto. O maior ganho no Gradle é o configuration cache. No Rust, é compilar só um ABI em dev.

**Gradle.** O `gradle.properties` gerado:

```properties
org.gradle.jvmargs=-Xmx4g -XX:+UseParallelGC -Dfile.encoding=UTF-8
org.gradle.caching=true
org.gradle.configuration-cache=true
org.gradle.parallel=true
kotlin.daemon.jvmargs=-Xmx2g
android.useAndroidX=true
android.nonTransitiveRClass=true
```

- Versões atuais de Gradle, AGP, Kotlin (K2) e BOM do Compose, fixadas no `libs.versions.toml`. O satchel está em Gradle 8.14, BOM `2024.10` e coroutines `1.9`, e vale atualizar junto.
- KSP em vez de kapt, se algum dia houver geração de código.
- Em dev, `installDebug` incremental, nunca `clean build`. No Studio, Apply Changes e os previews do Compose.
- `doctor --profile` roda `./gradlew assembleDebug --profile` e mostra onde o tempo foi.

**Rust.**

- **Um ABI em dev:** `x86_64` no emulador e `arm64-v8a` no celular. Os três ABIs só no `build --release`.
- **A task do Gradle que chama o `cargo-ndk` declara inputs e outputs.** Os inputs são `crates/**` e o `Cargo.lock`; o output é o `jniLibs/`. Assim ela fica up-to-date quando o Rust não mudou e entra no build cache.
- **Perfil `dev` com dependências otimizadas.** Com `[profile.dev.package."*"] opt-level = 2`, as dependências são compiladas uma vez e otimizadas, e o seu crate continua rápido de compilar.
- **`sccache`**, se instalado: o `doctor` sugere e o `dev` usa.
- **Um `target/` só no workspace**, compartilhado entre Android, iOS e os testes do host.

## Fases

Cada fase termina com algo usável. A fase 1 já melhora o satchel antes de qualquer CLI existir.

1. **Extrair o template do satchel.**
   - Trocar o `satchel-ffi` de `jni` para UniFFI e validar no emulador `satchel_api35`, com o `NativeCatalogTest` passando.
   - Aplicar o `gradle.properties` e a task com inputs e outputs da seção anterior. Medir o antes e o depois com `--profile`.
   - Criar `ios/project.yml` (XcodeGen) com um app SwiftUI mínimo chamando o core.
   - Copiar a estrutura para um diretório de template com placeholders de nome, pacote e bundle id.
2. **`new` e `doctor`.** Dar `new` num diretório vazio e chegar a um `cargo test` + `./gradlew installDebug` verde, sem editar nada à mão.
3. **`bindings` e `dev` para Android.** O watcher, a detecção de ABI e a reinstalação. A meta é menos de 10 s entre salvar um `.rs` e o app reaberto, com o cache quente.
4. **`dev` e `build` para iOS.** Só no Mac: XCFramework, `xcodebuild` e `simctl`. Em Linux, o `doctor` explica o que falta.
5. **Gerar o app do curupira com a CLI.** É o teste de verdade. Tudo o que precisar de ajuste à mão vira issue do template.
6. **`build --release` e `upgrade`.** Os três ABIs, a assinatura e o merge de três vias do template.

O `TESTING_STANDARD.md` vale para a própria CLI: `cargo fmt --check`, `clippy -D warnings` e um teste de integração que roda `new` + `cargo test` num diretório temporário.

## Riscos e decisões em aberto

- **O iOS depende do Mac.** O satchel já tem o app iOS parado em "Aberto no Mac". As fases 1 e 4 exigem tempo reservado na máquina com Xcode.
- **Mudanças no UniFFI.** A API de proc-macros muda entre versões. Fixe a versão no template e atualize só pelo `upgrade`.
- **Extensões do iOS** (File Provider e WidgetKit) precisam de targets próprios no `project.yml` e de um App Group. O template deve ter um exemplo comentado, porque o satchel e o curupira precisam disso.
- **Concorrência com o jffi**, que tem a mesma proposta. Vale ler o código dele antes da fase 2 e decidir se dá para contribuir em vez de começar do zero.

* [ ] Conferir se `jirau` está livre no crates.io e no GitHub
* [ ] Gerar o projeto Android também com o Amper, ou só com Gradle
* [ ] TUI no `new` (perguntas interativas) ou só flags

