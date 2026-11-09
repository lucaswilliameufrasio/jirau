#!/usr/bin/env bash
# ==============================================================================
# Confere se as versões fixadas em src/templates/versions.rs ainda são as
# estáveis mais recentes nos registros oficiais. Sai com código 1 se alguma
# estiver desatualizada (use no CI ou antes de cada release do jirau).
#
# Uso: scripts/check-template-versions.sh
# ==============================================================================
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
exec python3 - "$ROOT/src/templates/versions.rs" <<'PY'
import json, re, sys, urllib.request

src = open(sys.argv[1], encoding="utf-8").read()
pinned = dict(re.findall(r'pub const (\w+): &str = "([^"]+)"', src))
pinned.update({k: v for k, v in re.findall(r'pub const (\w+): u32 = (\d+);', src)})
UA = {"User-Agent": "jirau-template-version-check"}

def get(url):
    req = urllib.request.Request(url, headers=UA)
    with urllib.request.urlopen(req, timeout=30) as r:
        return r.read().decode()

def numeric(versions):
    stable = [v for v in versions if re.fullmatch(r"[0-9]+(\.[0-9]+)*", v)]
    return max(stable, key=lambda v: [int(p) for p in v.split(".")])

def maven(base, group, artifact):
    xml = get(f"{base}/{group.replace('.', '/')}/{artifact}/maven-metadata.xml")
    return numeric(re.findall(r"<version>([^<]+)</version>", xml))

GOOGLE = "https://dl.google.com/android/maven2"
CENTRAL = "https://repo1.maven.org/maven2"

def crate(name):
    return json.loads(get(f"https://crates.io/api/v1/crates/{name}"))["crate"]["max_stable_version"]

def gh_major(repo):
    tag = json.loads(get(f"https://api.github.com/repos/{repo}/releases/latest"))["tag_name"]
    return tag.lstrip("v").split(".")[0]

def android_repo(prefix):
    xml = get("https://dl.google.com/android/repository/repository2-3.xml")
    out = []
    for path, body in re.findall(r'<remotePackage path="(' + re.escape(prefix) + r'[^"]+)">(.*?)</remotePackage>', xml, re.S):
        if '<channelRef ref="channel-0"' in body:
            out.append(path[len(prefix):])
    return out

def semver_req_ok(req, latest):
    """Requisito do Cargo ("0.41", "1.29", "2") aceita a última versão?"""
    r = req.split(".")
    l = latest.split(".")
    if r[0] != l[0]:
        return False
    if r[0] == "0":
        return len(r) > 1 and r[1] == l[1]
    return True

checks = [
    ("GRADLE", lambda: json.loads(get("https://services.gradle.org/versions/current"))["version"], "eq"),
    ("AGP", lambda: maven(GOOGLE, "com.android.tools.build", "gradle"), "eq"),
    ("KOTLIN", lambda: maven(CENTRAL, "org.jetbrains.kotlin", "kotlin-gradle-plugin"), "eq"),
    ("COMPOSE_BOM", lambda: maven(GOOGLE, "androidx.compose", "compose-bom"), "eq"),
    ("ANDROIDX_CORE_KTX", lambda: maven(GOOGLE, "androidx.core", "core-ktx"), "eq"),
    ("ANDROIDX_LIFECYCLE", lambda: maven(GOOGLE, "androidx.lifecycle", "lifecycle-runtime-ktx"), "eq"),
    ("ANDROIDX_ACTIVITY_COMPOSE", lambda: maven(GOOGLE, "androidx.activity", "activity-compose"), "eq"),
    ("ANDROIDX_TEST_EXT_JUNIT", lambda: maven(GOOGLE, "androidx.test.ext", "junit"), "eq"),
    ("ANDROIDX_TEST_RUNNER", lambda: maven(GOOGLE, "androidx.test", "runner"), "eq"),
    ("JUNIT4", lambda: maven(CENTRAL, "junit", "junit"), "eq"),
    ("JNA", lambda: maven(CENTRAL, "net.java.dev.jna", "jna"), "eq"),
    ("KOTLINX_COROUTINES", lambda: maven(CENTRAL, "org.jetbrains.kotlinx", "kotlinx-coroutines-core"), "eq"),
    ("ANDROID_COMPILE_SDK", lambda: str(max(int(p.split(".")[0].split("-")[-1]) for p in android_repo("platforms;android-") if re.fullmatch(r"[0-9]+(\.[0-9]+)?", p))), "eq"),
    ("ANDROID_BUILD_TOOLS", lambda: numeric(android_repo("build-tools;")), "eq"),
    ("ANDROID_NDK", lambda: numeric(android_repo("ndk;")), "eq"),
    ("JAVA_LTS", lambda: str(json.loads(get("https://api.adoptium.net/v3/info/available_releases"))["most_recent_lts"]), "eq"),
    ("RUST", lambda: re.search(r'\[pkg\.rust\]\nversion = "([0-9.]+)', get("https://static.rust-lang.org/dist/channel-rust-stable.toml")).group(1), "eq"),
    ("UNIFFI", lambda: crate("uniffi"), "semver"),
    ("RUSQLITE", lambda: crate("rusqlite"), "semver"),
    ("SERDE", lambda: crate("serde"), "semver"),
    ("SERDE_JSON", lambda: crate("serde_json"), "semver"),
    ("THISERROR", lambda: crate("thiserror"), "semver"),
    ("UUID", lambda: crate("uuid"), "semver"),
    ("AWS_LC_RS", lambda: crate("aws-lc-rs"), "semver"),
    ("ACTION_CHECKOUT", lambda: gh_major("actions/checkout"), "major"),
    ("ACTION_SETUP_JAVA", lambda: gh_major("actions/setup-java"), "major"),
    ("ACTION_GRADLE", lambda: gh_major("gradle/actions"), "major"),
    ("ACTION_RUST_CACHE", lambda: gh_major("Swatinem/rust-cache"), "major"),
]

outdated = 0
for name, fetch, mode in checks:
    current = pinned[name]
    try:
        latest = fetch()
    except Exception as e:  # rede/registro indisponível não deve passar em silêncio
        print(f"?  {name:28} {current:24} (falha ao consultar: {e})")
        outdated += 1
        continue
    if mode == "semver":
        ok = semver_req_ok(current, latest)
    elif mode == "major":
        ok = current.rsplit("@v", 1)[-1] == latest
    else:
        ok = current == latest
    mark = "ok" if ok else "!!"
    print(f"{mark} {name:28} {current:24} última estável: {latest}")
    outdated += 0 if ok else 1

if outdated:
    print(f"\n{outdated} versão(ões) desatualizada(s) em src/templates/versions.rs")
    sys.exit(1)
print("\nTodas as versões dos templates estão atualizadas.")
PY
