

precious-respect

production



27 days or $5.00 left

Agent








BOTTER
Deployments
Variables
Metrics
Console
Settings
botter-production-4317.up.railway.app
EU West
1 Replica







HISTORY

Hide Skipped




























BOTTER
/
7bc862a5
Failed

2026-08-10 21:51 GMT+1
botter-production-4317.up.railway.app
Get Help
Details
Build Logs
Deploy Logs
Network Logs
Diagnosis › Try running a diagnosis to understand why this deployment failed.

Diagnose
Search build logs

You reached the start of the range
2026-08-10 21:46
scheduling build on Metal builder "builder-scehkg"
unpacking archive
310 KB
4ms
uploading snapshot
69.5 KB
16ms

internal
load build definition from sharklet/Dockerfile
0ms

internal
load metadata for docker.io/library/debian:bookworm-slim
402ms

internal
load metadata for docker.io/library/rust:1.88-slim
836ms

internal
load .dockerignore
0ms

builder
FROM docker.io/library/rust:1.88-slim@sha256:38bc5a86d998772d4aec2348656ed21438d20fcdce2795b56ca434cf21430d89
21ms

internal
load build context
0ms

stage-1
FROM docker.io/library/debian:bookworm-slim@sha256:abd67ffcfa541b485a3dff59865ab629aa048a6c613e639d36e7456b0b229241 cached
23ms

stage-1
RUN apt-get update && apt-get install -y --no-install-recommends     ca-certificates     && rm -rf /var/lib/apt/lists/*
6s
done.

builder
RUN apt-get update && apt-get install -y --no-install-recommends     build-essential pkg-config libssl-dev     && rm -rf /var/lib/apt/lists/*
7s
Processing triggers for libc-bin (2.36-9+deb12u10) ...

stage-1
WORKDIR /app
75ms

builder
WORKDIR /app
130ms

builder
COPY . .
91ms

builder
RUN cargo build --release 2>&1 | tee /tmp/build.log;     test -f target/release/sharklet || (echo "BUILD FAILED - dumping log" && cat /tmp/build.log && exit 1)
58s
    Updating crates.io index
     Locking 443 packages to latest compatible versions
      Adding generic-array v0.14.7 (available: v0.14.9)
      Adding r2d2_sqlite v0.24.0 (available: v0.35.0)
      Adding rand v0.8.7 (available: v0.10.2)
      Adding rusqlite v0.31.0 (available: v0.40.2)
      Adding solang-parser v0.3.3 (available: v0.3.5)
      Adding thiserror v1.0.69 (available: v2.0.20)
      Adding toml v0.8.23 (available: v1.1.4+spec-1.1.0)
      Adding warp v0.3.7 (available: v0.4.3)
 Downloading crates ...
  Downloaded byte-slice-cast v1.2.3
  Downloaded futures-core v0.3.33
  Downloaded powerfmt v0.2.0
  Downloaded scheduled-thread-pool v0.2.7
  Downloaded keccak v0.1.6
  Downloaded percent-encoding v2.3.2
  Downloaded time-core v0.1.9
  Downloaded tokio-tungstenite v0.20.1
  Downloaded tiny-keccak v2.0.2
  Downloaded home v0.5.12
  Downloaded subtle v2.6.1
  Downloaded signature v2.2.0
  Downloaded headers-core v0.2.0
  Downloaded tokio-tungstenite v0.21.0
  Downloaded tinystr v0.8.3
  Downloaded spin v0.9.9
  Downloaded new_debug_unreachable v1.0.6
  Downloaded tracing-futures v0.2.5
  Downloaded utf8_iter v1.0.4
  Downloaded try-lock v0.2.5
  Downloaded want v0.3.1
  Downloaded uint v0.9.5
  Downloaded uuid v0.8.2
  Downloaded walkdir v2.5.0
  Downloaded url v2.5.8
  Downloaded uuid v1.24.0
  Downloaded mio v1.2.2
  Downloaded warp v0.3.7
  Downloaded icu_collections v2.2.0
  Downloaded zerovec v0.11.6
  Downloaded idna v1.1.0
  Downloaded tracing-subscriber v0.3.23
  Downloaded h2 v0.3.27
  Downloaded winnow v1.0.4
  Downloaded webpki-roots v0.25.4
  Downloaded rustls v0.21.12
  Downloaded zerocopy v0.8.56
  Downloaded syn v2.0.119
  Downloaded vcpkg v0.2.15
  Downloaded regex-syntax v0.8.11
  Downloaded lalrpop v0.20.2
  Downloaded time v0.3.55
  Downloaded syn v3.0.3
  Downloaded rustix v1.1.4
  Downloaded hyper v0.14.32
  Downloaded tracing v0.1.44
  Downloaded syn v1.0.109
  Downloaded winnow v0.7.15
  Downloaded hashbrown v0.14.5
  Downloaded regex-automata v0.4.18
  Downloaded futures-util v0.3.33
  Downloaded icu_properties_data v2.2.0
  Downloaded hashbrown v0.17.1
  Downloaded rustls-webpki v0.101.7
  Downloaded proptest v1.11.0
  Downloaded serde_json v1.0.151
  Downloaded hashers v1.0.1
  Downloaded petgraph v0.6.5
  Downloaded rusqlite v0.31.0
  Downloaded typenum v1.20.1
  Downloaded chrono v0.4.45
  Downloaded zerotrie v0.2.4
  Downloaded icu_normalizer v2.2.0
  Downloaded icu_locale_core v2.2.0
  Downloaded sha3 v0.10.9
  Downloaded libc v0.2.189
  Downloaded http v1.5.0
  Downloaded http v0.2.12
  Downloaded tokio-util v0.7.19
  Downloaded tokio v1.53.1
  Downloaded bitvec v1.1.1
  Downloaded tungstenite v0.21.0
  Downloaded toml_edit v0.22.27
  Downloaded num-traits v0.2.19
  Downloaded num-bigint v0.4.8
  Downloaded memchr v2.8.3
  Downloaded icu_normalizer_data v2.2.0
  Downloaded headers v0.3.9
  Downloaded reqwest v0.11.27
  Downloaded regex v1.13.1
  Downloaded tungstenite v0.20.1
  Downloaded toml_edit v0.25.13+spec-1.1.0
  Downloaded parity-scale-codec v3.7.5
  Downloaded httparse v1.10.1
  Downloaded getrandom v0.3.4
  Downloaded encoding_rs v0.8.35
  Downloaded socket2 v0.6.5
  Downloaded sharded-slab v0.1.7
  Downloaded unicode-ident v1.0.24
  Downloaded tracing-core v0.1.36
  Downloaded parking_lot v0.12.5
  Downloaded once_cell v1.21.4
  Downloaded log v0.4.33
  Downloaded yoke v0.8.3
  Downloaded icu_properties v2.2.0
  Downloaded iana-time-zone v0.1.65
  Downloaded hmac v0.12.1
  Downloaded getrandom v0.4.3
  Downloaded pin-project v1.1.13
  Downloaded socket2 v0.5.10
  Downloaded unicase v2.9.0
  Downloaded tracing-attributes v0.1.31
  Downloaded ring v0.17.14
  Downloaded toml_parser v1.1.3+spec-1.1.0
  Downloaded password-hash v0.4.2
  Downloaded parking_lot_core v0.9.12
  Downloaded open-fastrlp v0.1.4
  Downloaded num_enum_derive v0.7.6
  Downloaded num_enum v0.7.6
  Downloaded num-integer v0.1.46
  Downloaded lock_api v0.4.14
  Downloaded litemap v0.8.2
  Downloaded lazy_static v1.5.0
  Downloaded zmij v1.0.23
  Downloaded zerovec-derive v0.11.3
  Downloaded zeroize v1.9.0
  Downloaded yansi v0.5.1
  Downloaded wyz v0.5.1
  Downloaded writeable v0.6.3
  Downloaded getrandom v0.2.17
  Downloaded toml v0.8.23
  Downloaded smallvec v1.15.2
  Downloaded rand v0.9.5
  Downloaded tracing-log v0.2.0
  Downloaded hashlink v0.9.1
  Downloaded glob v0.3.4
  Downloaded tinyvec v1.12.0
  Downloaded itertools v0.11.0
  Downloaded rand v0.10.2
  Downloaded ethers-core v2.0.14
  Downloaded aho-corasick v1.1.5
  Downloaded version_check v0.9.5
  Downloaded unarray v0.1.4
  Downloaded toml_datetime v1.1.1+spec-1.1.0
  Downloaded num_cpus v1.17.0
  Downloaded nu-ansi-term v0.50.3
  Downloaded multer v2.1.0
  Downloaded mime_guess v2.0.5
  Downloaded futures-task v0.3.33
  Downloaded futures-sink v0.3.33
  Downloaded zerofrom-derive v0.1.7
  Downloaded icu_provider v2.2.0
  Downloaded hyper-rustls v0.24.2
  Downloaded heck v0.5.0
  Downloaded group v0.13.0
  Downloaded tokio-rustls v0.24.1
  Downloaded time-macros v0.2.32
  Downloaded term v0.7.0
  Downloaded svm-rs v0.3.5
  Downloaded solang-parser v0.3.3
  Downloaded rayon v1.12.0
  Downloaded parity-scale-codec-derive v3.7.5
  Downloaded option-ext v0.2.0
  Downloaded open-fastrlp-derive v0.1.1
  Downloaded mime v0.3.17
  Downloaded httpdate v1.0.3
  Downloaded futures-timer v3.0.4
  Downloaded thiserror v2.0.20
  Downloaded spki v0.7.3
  Downloaded sha2 v0.10.9
  Downloaded utf-8 v0.7.6
  Downloaded untrusted v0.9.0
  Downloaded untrusted v0.7.1
  Downloaded unicode-xid v0.2.6
  Downloaded tracing-serde v0.2.0
  Downloaded tower-service v0.3.3
  Downloaded toml_write v0.1.2
  Downloaded num-conv v0.2.2
  Downloaded lalrpop-util v0.20.2
  Downloaded idna_adapter v1.2.2
  Downloaded futures-macro v0.3.33
  Downloaded zerofrom v0.1.8
  Downloaded yoke-derive v0.8.2
  Downloaded generic-array v0.14.7
  Downloaded slab v0.4.12
  Downloaded md-5 v0.10.6
  Downloaded http-body v0.4.6
  Downloaded hex v0.4.3
  Downloaded linux-raw-sys v0.12.1
  Downloaded fxhash v0.2.1
  Downloaded shlex v2.0.1
  Downloaded rlp v0.5.2
  Downloaded matchers v0.2.0
  Downloaded toml_datetime v0.6.11
  Downloaded synstructure v0.13.2
  Downloaded rlp-derive v0.1.0
  Downloaded signal-hook-registry v1.4.8
  Downloaded serde_urlencoded v0.7.1
  Downloaded proc-macro2 v1.0.107
  Downloaded indexmap v2.14.0
  Downloaded thiserror-impl v2.0.20
  Downloaded ryu v1.0.23
  Downloaded rand v0.8.7
  Downloaded ethers-solc v2.0.14
  Downloaded tempfile v3.27.0
  Downloaded sync_wrapper v0.1.2
  Downloaded static_assertions v1.1.0
  Downloaded sha1 v0.10.7
  Downloaded serde_spanned v0.6.9
  Downloaded tokio-macros v2.7.2
  Downloaded tinyvec_macros v0.1.1
  Downloaded phf_shared v0.11.3
  Downloaded phf_macros v0.11.3
  Downloaded konst v0.2.20
  Downloaded k256 v0.13.4
  Downloaded stable_deref_trait v1.2.1
  Downloaded siphasher v1.0.3
  Downloaded scoped-tls v1.0.1
  Downloaded rustls-pemfile v1.0.4
  Downloaded rayon-core v1.13.0
  Downloaded rand_chacha v0.9.0
  Downloaded phf_generator v0.11.3
  Downloaded phf v0.11.3
  Downloaded pem v1.1.1
  Downloaded pbkdf2 v0.11.0
  Downloaded thiserror-impl v1.0.69
  Downloaded thiserror v1.0.69
  Downloaded tap v1.0.1
  Downloaded string_cache v0.8.9
  Downloaded spin v0.5.2
  Downloaded serde v1.0.229
  Downloaded semver v1.0.28
  Downloaded rustc-hex v2.1.0
  Downloaded rand_chacha v0.3.1
  Downloaded r2d2 v0.8.10
  Downloaded quote v1.0.47
  Downloaded pbkdf2 v0.12.2
  Downloaded path-slash v0.2.1
  Downloaded konst_macro_rules v0.2.19
  Downloaded jsonwebtoken v8.3.0
  Downloaded impl-serde v0.4.0
  Downloaded thread_local v1.1.10
  Downloaded strum v0.26.3
  Downloaded simple_asn1 v0.6.4
  Downloaded serde_derive v1.0.229
  Downloaded sec1 v0.7.3
  Downloaded rustversion v1.0.23
  Downloaded ripemd v0.1.3
  Downloaded rfc6979 v0.4.0
  Downloaded rand_xorshift v0.4.0
  Downloaded rand_core v0.10.1
  Downloaded rand_core v0.6.4
  Downloaded radium v0.7.0
  Downloaded r2d2_sqlite v0.24.0
  Downloaded proc-macro-crate v3.5.0
  Downloaded primitive-types v0.12.2
  Downloaded ethers-providers v2.0.14
  Downloaded der v0.7.10
  Downloaded crypto-bigint v0.5.5
  Downloaded coins-bip39 v0.8.7
  Downloaded cc v1.4.2
  Downloaded aes v0.8.4
  Downloaded jobserver v0.1.35
  Downloaded itoa v1.0.18
  Downloaded ipnet v2.12.1
  Downloaded instant v0.1.13
  Downloaded inout v0.1.4
  Downloaded indenter v0.3.4
  Downloaded impl-trait-for-tuples v0.2.3
  Downloaded impl-rlp v0.3.0
  Downloaded impl-codec v0.6.0
  Downloaded strum_macros v0.26.4
  Downloaded serde_core v1.0.229
  Downloaded sct v0.7.1
  Downloaded scrypt v0.10.0
  Downloaded scopeguard v1.2.0
  Downloaded scale-info-derive v2.11.6
  Downloaded scale-info v2.11.6
  Downloaded same-file v1.0.6
  Downloaded salsa20 v0.10.2
  Downloaded rand_core v0.9.5
  Downloaded prettyplease v0.2.37
  Downloaded ppv-lite86 v0.2.21
  Downloaded pkg-config v0.3.33
  Downloaded pkcs8 v0.10.2
  Downloaded pin-project-lite v0.2.17
  Downloaded futures v0.3.33
  Downloaded fallible-iterator v0.3.0
  Downloaded ethers-contract-abigen v2.0.14
  Downloaded elliptic-curve v0.13.8
  Downloaded derive_more-impl v1.0.0
  Downloaded derive_more v1.0.0
  Downloaded crossbeam-utils v0.8.22
  Downloaded crossbeam-epoch v0.9.20
  Downloaded const_format v0.2.36
  Downloaded chacha20 v0.10.1
  Downloaded camino v1.2.5
  Downloaded bytes v1.12.1
  Downloaded bitflags v2.13.1
  Downloaded base64 v0.21.7
  Downloaded base64 v0.13.1
  Downloaded precomputed-hash v0.1.1
  Downloaded pin-project-internal v1.1.13
  Downloaded futures-locks v0.7.1
  Downloaded futures-channel v0.3.33
  Downloaded find-msvc-tools v0.1.10
  Downloaded ff v0.13.1
  Downloaded fastrand v2.5.0
  Downloaded eyre v0.6.12
  Downloaded ethers-signers v2.0.14
  Downloaded ethers-middleware v2.0.14
  Downloaded ring v0.16.20
  Downloaded ethers-etherscan v2.0.14
  Downloaded ethers-contract-derive v2.0.14
  Downloaded ethers-contract v2.0.14
  Downloaded ethabi v18.0.0
  Downloaded equivalent v1.0.2
  Downloaded enr v0.10.0
  Downloaded ena v0.14.4
  Downloaded either v1.17.0
  Downloaded ecdsa v0.16.9
  Downloaded dotenvy v0.15.7
  Downloaded libsqlite3-sys v0.28.0
  Downloaded displaydoc v0.2.7
  Downloaded digest v0.10.7
  Downloaded deranged v0.5.8
  Downloaded data-encoding v2.11.1
  Downloaded crossbeam-deque v0.8.7
  Downloaded const_format_proc_macros v0.2.34
  Downloaded const-oid v0.9.6
  Downloaded const-hex v1.19.1
  Downloaded coins-bip32 v0.8.7
  Downloaded cargo_metadata v0.18.1
  Downloaded bs58 v0.5.1
  Downloaded block-buffer v0.10.4
  Downloaded base64ct v1.8.3
  Downloaded ahash v0.8.12
  Downloaded potential_utf v0.1.5
  Downloaded futures-io v0.3.33
  Downloaded futures-executor v0.3.33
  Downloaded funty v2.0.0
  Downloaded fnv v1.0.7
  Downloaded fixedbitset v0.4.2
  Downloaded fixed-hash v0.8.0
  Downloaded fallible-streaming-iterator v0.1.9
  Downloaded ethers-addressbook v2.0.14
  Downloaded ethers v2.0.14
  Downloaded ethereum-types v0.14.1
  Downloaded dunce v1.0.5
  Downloaded dirs-next v2.0.0
  Downloaded dirs v5.0.1
  Downloaded ctr v0.9.2
  Downloaded crunchy v0.2.4
  Downloaded coins-core v0.8.7
  Downloaded cipher v0.4.4
  Downloaded byteorder v1.5.0
  Downloaded bit-vec v0.6.3
  Downloaded base16ct v0.2.0
  Downloaded auto_impl v1.3.0
  Downloaded async-trait v0.1.92
  Downloaded arrayvec v0.7.8
  Downloaded Inflector v0.11.4
  Downloaded fs2 v0.4.3
  Downloaded form_urlencoded v1.2.2
  Downloaded ethbloom v0.13.0
  Downloaded eth-keystore v0.5.0
  Downloaded errno v0.3.14
  Downloaded dirs-sys-next v0.1.2
  Downloaded dirs-sys v0.4.1
  Downloaded crypto-common v0.1.7
  Downloaded cpufeatures v0.3.0
  Downloaded cpufeatures v0.2.17
  Downloaded cfg-if v1.0.4
  Downloaded cargo-platform v0.1.9
  Downloaded bit-set v0.5.3
  Downloaded bech32 v0.9.1
  Downloaded autocfg v1.5.1
  Downloaded ascii-canvas v3.0.0
   Compiling proc-macro2 v1.0.107
   Compiling quote v1.0.47
   Compiling unicode-ident v1.0.24
   Compiling serde_core v1.0.229
   Compiling serde v1.0.229
   Compiling version_check v0.9.5
   Compiling cfg-if v1.0.4
   Compiling libc v0.2.189
   Compiling zeroize v1.9.0
   Compiling zerocopy v0.8.56
   Compiling typenum v1.20.1
   Compiling const-oid v0.9.6
   Compiling subtle v2.6.1
   Compiling hashbrown v0.17.1
   Compiling equivalent v1.0.2
   Compiling rustversion v1.0.23
   Compiling winnow v1.0.4
   Compiling toml_datetime v1.1.1+spec-1.1.0
   Compiling once_cell v1.21.4
   Compiling memchr v2.8.3
   Compiling itoa v1.0.18
   Compiling unicode-xid v0.2.6
   Compiling syn v1.0.109
   Compiling crunchy v0.2.4
   Compiling thiserror v1.0.69
   Compiling smallvec v1.15.2
   Compiling find-msvc-tools v0.1.10
   Compiling zmij v1.0.23
   Compiling shlex v2.0.1
   Compiling cpufeatures v0.2.17
   Compiling log v0.4.33
   Compiling pin-project-lite v0.2.17
   Compiling tiny-keccak v2.0.2
   Compiling stable_deref_trait v1.2.1
   Compiling autocfg v1.5.1
   Compiling byteorder v1.5.0
   Compiling cc v1.4.2
   Compiling parking_lot_core v0.9.12
   Compiling
 scopeguard v1.2.0
   Compiling hex v0.4.3
   Compiling generic-array v0.14.7
   Compiling futures-core v0.3.33
   Compiling futures-sink v0.3.33
   Compiling konst_macro_rules v0.2.19
   Compiling lock_api v0.4.14
   Compiling der v0.7.10
   Compiling rustc-hex v2.1.0
   Compiling serde_json v1.0.151
   Compiling writeable v0.6.3
   Compiling litemap v0.8.2
   Compiling rustix v1.1.4
   Compiling camino v1.2.5
   Compiling futures-channel v0.3.33
   Compiling konst v0.2.20
   Compiling byte-slice-cast v1.2.3
   Compiling
 futures-task v0.3.33
   Compiling icu_normalizer_data v2.2.0
   Compiling arrayvec v0.7.8
   Compiling fnv v1.0.7
   Compiling
 utf8_iter v1.0.4
   Compiling icu_properties_data v2.2.0
   Compiling num-traits v0.2.19
   Compiling tracing-core v0.1.36
   Compiling base16ct v0.2.0
   Compiling keccak v0.1.6
   Compiling indexmap v2.14.0
   Compiling httparse v1.10.1
   Compiling static_assertions v1.1.0
   Compiling slab v0.4.12
   Compiling futures-io v0.3.33
   Compiling rand_core v0.10.1
   Compiling aho-corasick v1.1.5
   Compiling heck v0.5.0
   Compiling regex-syntax v0.8.11
   Compiling getrandom v0.4.3
   Compiling base64 v0.21.7
   Compiling percent-encoding v2.3.2
   Compiling untrusted v0.9.0
   Compiling uint v0.9.5
   Compiling parity-scale-codec v3.7.5
   Compiling form_urlencoded v1.2.2
   Compiling bitflags v2.13.1
   Compiling rustls v0.21.12
   Compiling linux-raw-sys v0.12.1
   Compiling
 syn v3.0.3
   Compiling syn v2.0.119
   Compiling toml_parser v1.1.3+spec-1.1.0
   Compiling prettyplease v0.2.37
   Compiling const_format_proc_macros v0.2.34
   Compiling httpdate v1.0.3
   Compiling mime v0.3.17
   Compiling fastrand v2.5.0
   Compiling eyre v0.6.12
   Compiling crypto-common v0.1.7
   Compiling
 block-buffer v0.10.4
   Compiling getrandom v0.2.17
   Compiling digest v0.10.7
   Compiling
 toml_edit v0.25.13+spec-1.1.0
   Compiling ring v0.17.14
   Compiling errno v0.3.14
   Compiling socket2 v0.6.5
   Compiling mio v1.2.2
   Compiling iana-time-zone v0.1.65
   Compiling rand_core v0.6.4
   Compiling spki v0.7.3
   Compiling
 parking_lot v0.12.5
   Compiling hmac v0.12.1
   Compiling sha2 v0.10.9
   Compiling sha3 v0.10.9
   Compiling signal-hook-registry v1.4.8
   Compiling sec1 v0.7.3
   Compiling ff v0.13.1
   Compiling pkcs8 v0.10.2
   Compiling crypto-bigint v0.5.5
   Compiling rfc6979 v0.4.0
   Compiling signature v2.2.0
   Compiling group v0.13.0
   Compiling const_format v0.2.36
   Compiling sha1 v0.10.7
   Compiling try-lock v0.2.5
   Compiling proc-macro-crate v3.5.0
   Compiling want v0.3.1
   Compiling chrono v0.4.45
   Compiling socket2 v0.5.10
   Compiling inout v0.1.4
   Compiling const-hex v1.19.1
   Compiling utf-8 v0.7.6
   Compiling data-encoding v2.11.1
   Compiling tower-service v0.3.3
   Compiling
 cipher v0.4.4
   Compiling
 encoding_rs v0.8.35
   Compiling ryu v1.0.23
   Compiling num-conv v0.2.2
   Compiling thiserror v2.0.20
   Compiling time-core v0.1.9
   Compiling semver v1.0.28
   Compiling time-macros v0.2.32
   Compiling regex-automata v0.4.18
   Compiling ring v0.16.20
   Compiling num-integer v0.1.46
   Compiling webpki-roots v0.25.4
   Compiling lazy_static v1.5.0
   Compiling radium v0.7.0
   Compiling toml_write v0.1.2
   Compiling deranged v0.5.8
   Compiling winnow v0.7.15
   Compiling powerfmt v0.2.0
   Compiling
 num-bigint v0.4.8
   Compiling rlp-derive v0.1.0
   Compiling elliptic-curve v0.13.8
   Compiling
 bs58 v0.5.1
   Compiling ripemd v0.1.3
   Compiling tempfile v3.27.0
   Compiling ecdsa v0.16.9
   Compiling rustls-pemfile v1.0.4
   Compiling
 ahash v0.8.12
   Compiling
 tap v1.0.1
   Compiling sync_wrapper v0.1.2
   Compiling
 same-file v1.0.6
   Compiling untrusted v0.7.1
   Compiling bech32 v0.9.1
   Compiling ipnet v2.12.1
   Compiling spin v0.5.2
   Compiling indenter v0.3.4
   Compiling base64 v0.13.1
   Compiling k256 v0.13.4
   Compiling walkdir v2.5.0
   Compiling wyz v0.5.1
   Compiling salsa20 v0.10.2
   Compiling pbkdf2 v0.11.0
   Compiling pem v1.1.1
   Compiling fxhash v0.2.1
   Compiling pkg-config v0.3.33
   Compiling
 vcpkg v0.2.15
   Compiling
 funty v2.0.0
   Compiling
 dunce v1.0.5
   Compiling
 hashers v1.0.1
   Compiling scrypt v0.10.0
   Compiling time v0.3.55
   Compiling
 aes v0.8.4
   Compiling ctr v0.9.2
   Compiling pbkdf2 v0.12.2
   Compiling ppv-lite86 v0.2.21
   Compiling instant v0.1.13
   Compiling unicase v2.9.0
   Compiling futures-timer v3.0.4
   Compiling
 cpufeatures v0.3.0
   Compiling
 multer v2.1.0
   Compiling chacha20 v0.10.1
   Compiling
 scheduled-thread-pool v0.2.7
   Compiling futures-locks v0.7.1
   Compiling
 bitvec v1.1.1
   Compiling mime_guess v2.0.5
   Compiling
 fallible-streaming-iterator v0.1.9
   Compiling rand_chacha v0.3.1
   Compiling rand v0.10.2
   Compiling
 libsqlite3-sys v0.28.0
   Compiling synstructure v0.13.2
   Compiling fallible-iterator v0.3.0
   Compiling spin v0.9.9
   Compiling r2d2 v0.8.10
   Compiling hashbrown v0.14.5
   Compiling rand v0.8.7
   Compiling sharded-slab v0.1.7
   Compiling tracing-log v0.2.0
   Compiling thread_local v1.1.10
   Compiling scoped-tls v1.0.1
   Compiling nu-ansi-term v0.50.3
   Compiling dotenvy v0.15.7
   Compiling serde_derive v1.0.229
   Compiling
 displaydoc v0.2.7
   Compiling tokio-macros v2.7.2
   Compiling thiserror-impl v2.0.20
   Compiling async-trait v0.1.92
   Compiling regex v1.13.1
   Compiling uuid v1.24.0
   Compiling parity-scale-codec-derive v3.7.5
   Compiling impl-trait-for-tuples v0.2.3
   Compiling thiserror-impl v1.0.69
   Compiling derive_more-impl v1.0.0
   Compiling zerofrom-derive v0.1.7
   Compiling scale-info-derive v2.11.6
   Compiling yoke-derive v0.8.2
   Compiling zerovec-derive v0.11.3
   Compiling futures-macro v0.3.33
   Compiling tracing-attributes v0.1.31
   Compiling auto_impl v1.3.0
   Compiling strum_macros v0.26.4
   Compiling num_enum_derive v0.7.6
   Compiling fixed-hash v0.8.0
   Compiling pin-project-internal v1.1.13
   Compiling Inflector v0.11.4
   Compiling hashlink v0.9.1
   Compiling futures-util v0.3.33
   Compiling zerofrom v0.1.8
   Compiling derive_more v1.0.0
   Compiling num_enum v0.7.6
   Compiling yoke v0.8.3
   Compiling matchers v0.2.0
   Compiling tracing v0.1.44
   Compiling pin-project v1.1.13
   Compiling simple_asn1 v0.6.4
   Compiling zerovec v0.11.6
   Compiling zerotrie v0.2.4
   Compiling tracing-futures v0.2.5
   Compiling strum v0.26.3
   Compiling tinystr v0.8.3
   Compiling potential_utf v0.1.5
   Compiling icu_collections v2.2.0
   Compiling icu_locale_core v2.2.0
   Compiling bytes v1.12.1
   Compiling impl-serde v0.4.0
   Compiling serde_spanned v0.6.9
   Compiling cargo-platform v0.1.9
   Compiling toml_datetime v0.6.11
   Compiling serde_urlencoded v0.7.1
   Compiling coins-core v0.8.7
   Compiling uuid v0.8.2
   Compiling tracing-serde v0.2.0
   Compiling cargo_metadata v0.18.1
   Compiling toml_edit v0.22.27
   Compiling tracing-subscriber v0.3.23
   Compiling eth-keystore v0.5.0
   Compiling rlp v0.5.2
   Compiling coins-bip32 v0.8.7
   Compiling impl-codec v0.6.0
   Compiling scale-info v2.11.6
   Compiling jsonwebtoken v8.3.0
   Compiling open-fastrlp-derive v0.1.1
   Compiling icu_provider v2.2.0
   Compiling http v0.2.12
   Compiling tokio v1.53.1
   Compiling http v1.5.0
   Compiling
 impl-rlp v0.3.0
   Compiling icu_properties v2.2.0
   Compiling icu_normalizer v2.2.0
   Compiling enr v0.10.0
   Compiling ethbloom v0.13.0
   Compiling primitive-types v0.12.2
   Compiling coins-bip39 v0.8.7
   Compiling futures-executor v0.3.33
   Compiling futures v0.3.33
   Compiling http-body v0.4.6
   Compiling headers-core v0.2.0
   Compiling headers v0.3.9
   Compiling toml v0.8.23
   Compiling idna_adapter v1.2.2
   Compiling idna v1.1.0
   Compiling ethereum-types v0.14.1
   Compiling url v2.5.8
   Compiling ethabi v18.0.0
   Compiling open-fastrlp v0.1.4
   Compiling tungstenite v0.21.0
   Compiling sct v0.7.1
   Compiling rustls-webpki v0.101.7
   Compiling ethers-core v2.0.14
   Compiling tokio-util v0.7.19
   Compiling tokio-tungstenite v0.21.0
   Compiling tokio-rustls v0.24.1
   Compiling tungstenite v0.20.1
   Compiling h2 v0.3.27
   Compiling tokio-tungstenite v0.20.1
   Compiling ethers-contract-abigen v2.0.14
   Compiling ethers-contract-derive v2.0.14
   Compiling ethers-signers v2.0.14
   Compiling ethers-addressbook v2.0.14
   Compiling hyper v0.14.32
   Compiling hyper-rustls v0.24.2
   Compiling warp v0.3.7
   Compiling reqwest v0.11.27
   Compiling ethers-providers v2.0.14
   Compiling ethers-etherscan v2.0.14
   Compiling ethers-contract v2.0.14
   Compiling ethers-middleware v2.0.14
   Compiling ethers v2.0.14
   Compiling rusqlite v0.31.0
   Compiling r2d2_sqlite v0.24.0
   Compiling sharklet v0.2.0 (/app)
error: mismatched closing delimiter: `)`

   --> src/executor.rs:117:134
    |
117 | ..._per_gas: f64) -> Result<ExecutionResult, ExecutorError> {
    |                                                             ^ unclosed delimiter
...
143 | ...s_u128() as f64 / 1e18 * gas_priceself, amount: U256) -> Result<U256, ExecutorError> {
    |                                                        ^ mismatched closing delimiter
error: could not compile `sharklet` (bin "sharklet") due to 1 previous error

BUILD FAILED - dumping log
    Updating crates.io index
     Locking 443 packages to latest compatible versions
      Adding generic-array v0.14.7 (available: v0.14.9)
      Adding r2d2_sqlite v0.24.0 (available: v0.35.0)
      Adding rand v0.8.7 (available: v0.10.2)
      Adding rusqlite v0.31.0 (available: v0.40.2)
      Adding solang-parser v0.3.3 (available: v0.3.5)
      Adding thiserror v1.0.69 (available: v2.0.20)
      Adding toml v0.8.23 (available: v1.1.4+spec-1.1.0)
      Adding warp v0.3.7 (available: v0.4.3)
 Downloading crates ...
  Downloaded byte-slice-cast v1.2.3
  Downloaded futures-core v0.3.33
  Downloaded powerfmt v0.2.0
  Downloaded scheduled-thread-pool v0.2.7
  Downloaded keccak v0.1.6
  Downloaded percent-encoding v2.3.2
  Downloaded time-core v0.1.9
  Downloaded tokio-tungstenite v0.20.1
  Downloaded tiny-keccak v2.0.2
  Downloaded home v0.5.12
  Downloaded subtle v2.6.1
  Downloaded signature v2.2.0
  Downloaded headers-core v0.2.0
  Downloaded tokio-tungstenite v0.21.0
  Downloaded tinystr v0.8.3
  Downloaded spin v0.9.9
  Downloaded new_debug_unreachable v1.0.6
  Downloaded tracing-futures v0.2.5
  Downloaded utf8_iter v1.0.4
  Downloaded try-lock v0.2.5
  Downloaded want v0.3.1
  Downloaded uint v0.9.5
  Downloaded uuid v0.8.2
  Downloaded walkdir v2.5.0
  Downloaded url v2.5.8
  Downloaded uuid v1.24.0
  Downloaded mio v1.2.2
  Downloaded warp v0.3.7
  Downloaded icu_collections v2.2.0
  Downloaded zerovec v0.11.6
  Downloaded idna v1.1.0
  Downloaded tracing-subscriber v0.3.23
  Downloaded h2 v0.3.27
  Downloaded winnow v1.0.4
  Downloaded webpki-roots v0.25.4
  Downloaded rustls v0.21.12
  Downloaded zerocopy v0.8.56
  Downloaded syn v2.0.119
  Downloaded vcpkg v0.2.15
  Downloaded regex-syntax v0.8.11
  Downloaded lalrpop v0.20.2
  Downloaded time v0.3.55
  Downloaded syn v3.0.3
  Downloaded rustix v1.1.4
  Downloaded hyper v0.14.32
  Downloaded tracing v0.1.44
  Downloaded syn v1.0.109
  Downloaded winnow v0.7.15
  Downloaded hashbrown v0.14.5
  Downloaded regex-automata v0.4.18
  Downloaded futures-util v0.3.33
  Downloaded icu_properties_data v2.2.0
  Downloaded hashbrown v0.17.1
  Downloaded rustls-webpki v0.101.7
  Downloaded proptest v1.11.0
  Downloaded serde_json v1.0.151
  Downloaded hashers v1.0.1
  Downloaded petgraph v0.6.5
  Downloaded rusqlite v0.31.0
  Downloaded typenum v1.20.1
  Downloaded chrono v0.4.45
  Downloaded zerotrie v0.2.4
  Downloaded icu_normalizer v2.2.0
  Downloaded icu_locale_core v2.2.0
  Downloaded sha3 v0.10.9
  Downloaded libc v0.2.189
  Downloaded http v1.5.0
  Downloaded http v0.2.12
  Downloaded tokio-util v0.7.19
  Downloaded tokio v1.53.1
  Downloaded bitvec v1.1.1
  Downloaded tungstenite v0.21.0
  Downloaded toml_edit v0.22.27
  Downloaded num-traits v0.2.19
  Downloaded num-bigint v0.4.8
  Downloaded memchr v2.8.3
  Downloaded icu_normalizer_data v2.2.0
  Downloaded headers v0.3.9
  Downloaded reqwest v0.11.27
  Downloaded regex v1.13.1
  Downloaded tungstenite v0.20.1
  Downloaded toml_edit v0.25.13+spec-1.1.0
  Downloaded parity-scale-codec v3.7.5
  Downloaded httparse v1.10.1
  Downloaded getrandom v0.3.4
  Downloaded encoding_rs v0.8.35
  Downloaded socket2 v0.6.5
  Downloaded sharded-slab v0.1.7
  Downloaded unicode-ident v1.0.24
  Downloaded tracing-core v0.1.36
  Downloaded parking_lot v0.12.5
  Downloaded once_cell v1.21.4
  Downloaded log v0.4.33
  Downloaded yoke v0.8.3
  Downloaded icu_properties v2.2.0
  Downloaded iana-time-zone v0.1.65
  Downloaded hmac v0.12.1
  Downloaded getrandom v0.4.3
  Downloaded pin-project v1.1.13
  Downloaded socket2 v0.5.10
  Downloaded unicase v2.9.0
  Downloaded tracing-attributes v0.1.31
  Downloaded ring v0.17.14
  Downloaded toml_parser v1.1.3+spec-1.1.0
  Downloaded password-hash v0.4.2
  Downloaded parking_lot_core v0.9.12
  Downloaded open-fastrlp v0.1.4
  Downloaded num_enum_derive v0.7.6
  Downloaded num_enum v0.7.6
  Downloaded num-integer v0.1.46
  Downloaded lock_api v0.4.14
  Downloaded litemap v0.8.2
  Downloaded lazy_static v1.5.0
  Downloaded zmij v1.0.23
  Downloaded zerovec-derive v0.11.3
  Downloaded zeroize v1.9.0
  Downloaded yansi v0.5.1
  Downloaded wyz v0.5.1
  Downloaded writeable v0.6.3
  Downloaded getrandom v0.2.17
  Downloaded toml v0.8.23
  Downloaded smallvec v1.15.2
  Downloaded rand v0.9.5
  Downloaded tracing-log v0.2.0
  Downloaded hashlink v0.9.1
  Downloaded glob v0.3.4
  Downloaded tinyvec v1.12.0
  Downloaded itertools v0.11.0
  Downloaded rand v0.10.2
  Downloaded ethers-core v2.0.14
  Downloaded aho-corasick v1.1.5
  Downloaded version_check v0.9.5
  Downloaded unarray v0.1.4
  Downloaded toml_datetime v1.1.1+spec-1.1.0
  Downloaded num_cpus v1.17.0
  Downloaded nu-ansi-term v0.50.3
  Downloaded multer v2.1.0
  Downloaded mime_guess v2.0.5
  Downloaded futures-task v0.3.33
  Downloaded futures-sink v0.3.33
  Downloaded zerofrom-derive v0.1.7
  Downloaded icu_provider v2.2.0
  Downloaded hyper-rustls v0.24.2
  Downloaded heck v0.5.0
  Downloaded group v0.13.0
  Downloaded tokio-rustls v0.24.1
  Downloaded time-macros v0.2.32
  Downloaded term v0.7.0
  Downloaded svm-rs v0.3.5
  Downloaded solang-parser v0.3.3
  Downloaded rayon v1.12.0
  Downloaded parity-scale-codec-derive v3.7.5
  Downloaded option-ext v0.2.0
  Downloaded open-fastrlp-derive v0.1.1
  Downloaded mime v0.3.17
  Downloaded httpdate v1.0.3
  Downloaded futures-timer v3.0.4
  Downloaded thiserror v2.0.20
  Downloaded spki v0.7.3
  Downloaded sha2 v0.10.9
  Downloaded utf-8 v0.7.6
  Downloaded untrusted v0.9.0
  Downloaded untrusted v0.7.1
  Downloaded unicode-xid v0.2.6
  Downloaded tracing-serde v0.2.0
  Downloaded tower-service v0.3.3
  Downloaded toml_write v0.1.2
  Downloaded num-conv v0.2.2
  Downloaded lalrpop-util v0.20.2
  Downloaded idna_adapter v1.2.2
  Downloaded futures-macro v0.3.33
  Downloaded zerofrom v0.1.8
  Downloaded yoke-derive v0.8.2
  Downloaded generic-array v0.14.7
  Downloaded slab v0.4.12
  Downloaded md-5 v0.10.6
  Downloaded http-body v0.4.6
  Downloaded hex v0.4.3
  Downloaded linux-raw-sys v0.12.1
  Downloaded fxhash v0.2.1
  Downloaded shlex v2.0.1
  Downloaded rlp v0.5.2
  Downloaded matchers v0.2.0
  Downloaded toml_datetime v0.6.11
  Downloaded synstructure v0.13.2
  Downloaded rlp-derive v0.1.0
  Downloaded signal-hook-registry v1.4.8
  Downloaded serde_urlencoded v0.7.1
  Downloaded proc-macro2 v1.0.107
  Downloaded indexmap v2.14.0
  Downloaded thiserror-impl v2.0.20
  Downloaded ryu v1.0.23
  Downloaded rand v0.8.7
  Downloaded ethers-solc v2.0.14
  Downloaded tempfile v3.27.0
  Downloaded sync_wrapper v0.1.2
  Downloaded static_assertions v1.1.0
  Downloaded sha1 v0.10.7
  Downloaded serde_spanned v0.6.9
  Downloaded tokio-macros v2.7.2
  Downloaded tinyvec_macros v0.1.1
  Downloaded phf_shared v0.11.3
  Downloaded phf_macros v0.11.3
  Downloaded konst v0.2.20
  Downloaded k256 v0.13.4
  Downloaded stable_deref_trait v1.2.1
  Downloaded siphasher v1.0.3
  Downloaded scoped-tls v1.0.1
  Downloaded rustls-pemfile v1.0.4
  Downloaded rayon-core v1.13.0
  Downloaded rand_chacha v0.9.0
  Downloaded phf_generator v0.11.3
  Downloaded phf v0.11.3
  Downloaded pem v1.1.1
  Downloaded pbkdf2 v0.11.0
  Downloaded thiserror-impl v1.0.69
  Downloaded thiserror v1.0.69
  Downloaded tap v1.0.1
  Downloaded string_cache v0.8.9
  Downloaded spin v0.5.2
  Downloaded serde v1.0.229
  Downloaded semver v1.0.28
  Downloaded rustc-hex v2.1.0
  Downloaded rand_chacha v0.3.1
  Downloaded r2d2 v0.8.10
  Downloaded quote v1.0.47
  Downloaded pbkdf2 v0.12.2
  Downloaded path-slash v0.2.1
  Downloaded konst_macro_rules v0.2.19
  Downloaded jsonwebtoken v8.3.0
  Downloaded impl-serde v0.4.0
  Downloaded thread_local v1.1.10
  Downloaded strum v0.26.3
  Downloaded simple_asn1 v0.6.4
  Downloaded serde_derive v1.0.229
  Downloaded sec1 v0.7.3
  Downloaded rustversion v1.0.23
  Downloaded ripemd v0.1.3
  Downloaded rfc6979 v0.4.0
  Downloaded rand_xorshift v0.4.0
  Downloaded rand_core v0.10.1
  Downloaded rand_core v0.6.4
  Downloaded radium v0.7.0
  Downloaded r2d2_sqlite v0.24.0
  Downloaded proc-macro-crate v3.5.0
  Downloaded primitive-types v0.12.2
  Downloaded ethers-providers v2.0.14
  Downloaded der v0.7.10
  Downloaded crypto-bigint v0.5.5
  Downloaded coins-bip39 v0.8.7
  Downloaded cc v1.4.2
  Downloaded aes v0.8.4
  Downloaded jobserver v0.1.35
  Downloaded itoa v1.0.18
  Downloaded ipnet v2.12.1
  Downloaded instant v0.1.13
  Downloaded inout v0.1.4
  Downloaded indenter v0.3.4
  Downloaded impl-trait-for-tuples v0.2.3
  Downloaded impl-rlp v0.3.0
  Downloaded impl-codec v0.6.0
  Downloaded strum_macros v0.26.4
  Downloaded serde_core v1.0.229
  Downloaded sct v0.7.1
  Downloaded scrypt v0.10.0
  Downloaded scopeguard v1.2.0
  Downloaded scale-info-derive v2.11.6
  Downloaded scale-info v2.11.6
  Downloaded same-file v1.0.6
  Downloaded salsa20 v0.10.2
  Downloaded rand_core v0.9.5
  Downloaded prettyplease v0.2.37
  Downloaded ppv-lite86 v0.2.21
  Downloaded pkg-config v0.3.33
  Downloaded pkcs8 v0.10.2
  Downloaded pin-project-lite v0.2.17
  Downloaded futures v0.3.33
  Downloaded fallible-iterator v0.3.0
  Downloaded ethers-contract-abigen v2.0.14
  Downloaded elliptic-curve v0.13.8
  Downloaded derive_more-impl v1.0.0
  Downloaded derive_more v1.0.0
  Downloaded crossbeam-utils v0.8.22
  Downloaded crossbeam-epoch v0.9.20
  Downloaded const_format v0.2.36
  Downloaded chacha20 v0.10.1
  Downloaded camino v1.2.5
  Downloaded bytes v1.12.1
  Downloaded bitflags v2.13.1
  Downloaded base64 v0.21.7
  Downloaded base64 v0.13.1
  Downloaded precomputed-hash v0.1.1
  Downloaded pin-project-internal v1.1.13
  Downloaded futures-locks v0.7.1
  Downloaded futures-channel v0.3.33
  Downloaded find-msvc-tools v0.1.10
  Downloaded ff v0.13.1
  Downloaded fastrand v2.5.0
  Downloaded eyre v0.6.12
  Downloaded ethers-signers v2.0.14
  Downloaded ethers-middleware v2.0.14
  Downloaded ring v0.16.20
  Downloaded ethers-etherscan v2.0.14
  Downloaded ethers-contract-derive v2.0.14
  Downloaded ethers-contract v2.0.14
  Downloaded ethabi v18.0.0
  Downloaded equivalent v1.0.2
  Downloaded enr v0.10.0
  Downloaded ena v0.14.4
  Downloaded either v1.17.0
  Downloaded ecdsa v0.16.9
  Downloaded dotenvy v0.15.7
  Downloaded libsqlite3-sys v0.28.0
  Downloaded displaydoc v0.2.7
  Downloaded digest v0.10.7
  Downloaded deranged v0.5.8
  Downloaded data-encoding v2.11.1
  Downloaded crossbeam-deque v0.8.7
  Downloaded const_format_proc_macros v0.2.34
  Downloaded const-oid v0.9.6
  Downloaded const-hex v1.19.1
  Downloaded coins-bip32 v0.8.7
  Downloaded cargo_metadata v0.18.1
  Downloaded bs58 v0.5.1
  Downloaded block-buffer v0.10.4
  Downloaded base64ct v1.8.3
  Downloaded ahash v0.8.12
  Downloaded potential_utf v0.1.5
  Downloaded futures-io v0.3.33
  Downloaded futures-executor v0.3.33
  Downloaded funty v2.0.0
  Downloaded fnv v1.0.7
  Downloaded fixedbitset v0.4.2
  Downloaded fixed-hash v0.8.0
  Downloaded fallible-streaming-iterator v0.1.9
  Downloaded ethers-addressbook v2.0.14
  Downloaded ethers v2.0.14
  Downloaded ethereum-types v0.14.1
  Downloaded dunce v1.0.5
  Downloaded dirs-next v2.0.0
  Downloaded dirs v5.0.1
  Downloaded ctr v0.9.2
  Downloaded crunchy v0.2.4
  Downloaded coins-core v0.8.7
  Downloaded cipher v0.4.4
  Downloaded byteorder v1.5.0
  Downloaded bit-vec v0.6.3
  Downloaded base16ct v0.2.0
  Downloaded auto_impl v1.3.0
  Downloaded async-trait v0.1.92
  Downloaded arrayvec v0.7.8
  Downloaded Inflector v0.11.4
  Downloaded fs2 v0.4.3
  Downloaded form_urlencoded v1.2.2
  Downloaded ethbloom v0.13.0
  Downloaded eth-keystore v0.5.0
  Downloaded errno v0.3.14
  Downloaded dirs-sys-next v0.1.2
  Downloaded dirs-sys v0.4.1
  Downloaded crypto-common v0.1.7
  Downloaded cpufeatures v0.3.0
  Downloaded cpufeatures v0.2.17
  Downloaded cfg-if v1.0.4
  Downloaded cargo-platform v0.1.9
  Downloaded bit-set v0.5.3
  Downloaded bech32 v0.9.1
  Downloaded autocfg v1.5.1
  Downloaded ascii-canvas v3.0.0
   Compiling proc-macro2 v1.0.107
   Compiling quote v1.0.47
   Compiling unicode-ident v1.0.24
   Compiling serde_core v1.0.229
   Compiling serde v1.0.229
   Compiling version_check v0.9.5
   Compiling cfg-if v1.0.4
   Compiling libc v0.2.189
   Compiling zeroize v1.9.0
   Compiling zerocopy v0.8.56
   Compiling typenum v1.20.1
   Compiling const-oid v0.9.6
   Compiling subtle v2.6.1
   Compiling hashbrown v0.17.1
   Compiling equivalent v1.0.2
   Compiling rustversion v1.0.23
   Compiling winnow v1.0.4
   Compiling toml_datetime v1.1.1+spec-1.1.0
   Compiling once_cell v1.21.4
   Compiling memchr v2.8.3
   Compiling itoa v1.0.18
   Compiling unicode-xid v0.2.6
   Compiling syn v1.0.109
   Compiling crunchy v0.2.4
   Compiling thiserror v1.0.69
   Compiling smallvec v1.15.2
   Compiling find-msvc-tools v0.1.10
   Compiling zmij v1.0.23
   Compiling shlex v2.0.1
   Compiling cpufeatures v0.2.17
   Compiling log v0.4.33
   Compiling pin-project-lite v0.2.17
   Compiling tiny-keccak v2.0.2
   Compiling stable_deref_trait v1.2.1
   Compiling autocfg v1.5.1
   Compiling byteorder v1.5.0
   Compiling cc v1.4.2
   Compiling parking_lot_core v0.9.12
   Compiling scopeguard v1.2.0
   Compiling hex v0.4.3
   Compiling generic-array v0.14.7
   Compiling futures-core v0.3.33
   Compiling futures-sink v0.3.33
   Compiling konst_macro_rules v0.2.19
   Compiling lock_api v0.4.14
   Compiling der v0.7.10
   Compiling rustc-hex v2.1.0
   Compiling serde_json v1.0.151
   Compiling writeable v0.6.3
   Compiling litemap v0.8.2
   Compiling rustix v1.1.4
   Compiling camino v1.2.5
   Compiling futures-channel v0.3.33
   Compiling konst v0.2.20
   Compiling byte-slice-cast v1.2.3
   Compiling futures-task v0.3.33
   Compiling icu_normalizer_data v2.2.0
   Compiling arrayvec v0.7.8
   Compiling fnv v1.0.7
   Compiling utf8_iter v1.0.4
   Compiling icu_properties_data v2.2.0
   Compiling num-traits v0.2.19
   Compiling tracing-core v0.1.36
   Compiling base16ct v0.2.0
   Compiling keccak v0.1.6
   Compiling indexmap v2.14.0
   Compiling httparse v1.10.1
   Compiling static_assertions v1.1.0
   Compiling slab v0.4.12
   Compiling futures-io v0.3.33
   Compiling rand_core v0.10.1
   Compiling aho-corasick v1.1.5
   Compiling heck v0.5.0
   Compiling regex-syntax v0.8.11
   Compiling getrandom v0.4.3
   Compiling base64 v0.21.7
   Compiling percent-encoding v2.3.2
   Compiling untrusted v0.9.0
   Compiling uint v0.9.5
   Compiling parity-scale-codec v3.7.5
   Compiling form_urlencoded v1.2.2
   Compiling bitflags v2.13.1
   Compiling rustls v0.21.12
   Compiling linux-raw-sys v0.12.1
   Compiling syn v3.0.3
   Compiling syn v2.0.119
   Compiling toml_parser v1.1.3+spec-1.1.0
   Compiling prettyplease v0.2.37
   Compiling const_format_proc_macros v0.2.34
   Compiling httpdate v1.0.3
   Compiling mime v0.3.17
   Compiling fastrand v2.5.0
   Compiling eyre v0.6.12
   Compiling crypto-common v0.1.7
   Compiling block-buffer v0.10.4
   Compiling getrandom v0.2.17
   Compiling digest v0.10.7
   Compiling toml_edit v0.25.13+spec-1.1.0
   Compiling ring v0.17.14
   Compiling errno v0.3.14
   Compiling socket2 v0.6.5
   Compiling mio v1.2.2
   Compiling iana-time-zone v0.1.65
   Compiling rand_core v0.6.4
   Compiling spki v0.7.3
   Compiling parking_lot v0.12.5
   Compiling hmac v0.12.1
   Compiling sha2 v0.10.9
   Compiling sha3 v0.10.9
   Compiling signal-hook-registry v1.4.8
   Compiling sec1 v0.7.3
   Compiling ff v0.13.1
   Compiling pkcs8 v0.10.2
   Compiling crypto-bigint v0.5.5
   Compiling rfc6979 v0.4.0
   Compiling signature v2.2.0
   Compiling group v0.13.0
   Compiling const_format v0.2.36
   Compiling sha1 v0.10.7
   Compiling try-lock v0.2.5
   Compiling proc-macro-crate v3.5.0
   Compiling want v0.3.1
   Compiling chrono v0.4.45
   Compiling socket2 v0.5.10
   Compiling inout v0.1.4
   Compiling const-hex v1.19.1
   Compiling utf-8 v0.7.6
   Compiling data-encoding v2.11.1
   Compiling tower-service v0.3.3
   Compiling cipher v0.4.4
   Compiling encoding_rs v0.8.35
   Compiling ryu v1.0.23
   Compiling num-conv v0.2.2
   Compiling thiserror v2.0.20
   Compiling time-core v0.1.9
   Compiling semver v1.0.28
   Compiling time-macros v0.2.32
   Compiling regex-automata v0.4.18
   Compiling ring v0.16.20
   Compiling num-integer v0.1.46
   Compiling webpki-roots v0.25.4
   Compiling lazy_static v1.5.0
   Compiling radium v0.7.0
   Compiling toml_write v0.1.2
   Compiling deranged v0.5.8
   Compiling winnow v0.7.15
   Compiling powerfmt v0.2.0
   Compiling num-bigint v0.4.8
   Compiling rlp-derive v0.1.0
   Compiling elliptic-curve v0.13.8
   Compiling bs58 v0.5.1
   Compiling ripemd v0.1.3
   Compiling tempfile v3.27.0
   Compiling ecdsa v0.16.9
   Compiling rustls-pemfile v1.0.4
   Compiling ahash v0.8.12
   Compiling tap v1.0.1
   Compiling sync_wrapper v0.1.2
   Compiling same-file v1.0.6
   Compiling untrusted v0.7.1
   Compiling bech32 v0.9.1
   Compiling ipnet v2.12.1
   Compiling spin v0.5.2
   Compiling indenter v0.3.4
   Compiling base64 v0.13.1
   Compiling k256 v0.13.4
   Compiling walkdir v2.5.0
   Compiling wyz v0.5.1
   Compiling salsa20 v0.10.2
   Compiling pbkdf2 v0.11.0
   Compiling pem v1.1.1
   Compiling fxhash v0.2.1
   Compiling pkg-config v0.3.33
   Compiling vcpkg v0.2.15
   Compiling funty v2.0.0
   Compiling dunce v1.0.5
   Compiling hashers v1.0.1
   Compiling scrypt v0.10.0
   Compiling time v0.3.55
   Compiling aes v0.8.4
   Compiling ctr v0.9.2
   Compiling pbkdf2 v0.12.2
   Compiling ppv-lite86 v0.2.21
   Compiling instant v0.1.13
   Compiling unicase v2.9.0
   Compiling futures-timer v3.0.4
   Compiling cpufeatures v0.3.0
   Compiling multer v2.1.0
   Compiling chacha20 v0.10.1
   Compiling scheduled-thread-pool v0.2.7
   Compiling futures-locks v0.7.1
   Compiling bitvec v1.1.1
   Compiling mime_guess v2.0.5
   Compiling fallible-streaming-iterator v0.1.9
   Compiling rand_chacha v0.3.1
   Compiling rand v0.10.2
   Compiling libsqlite3-sys v0.28.0
   Compiling synstructure v0.13.2
   Compiling fallible-iterator v0.3.0
   Compiling spin v0.9.9
   Compiling r2d2 v0.8.10
   Compiling hashbrown v0.14.5
   Compiling rand v0.8.7
   Compiling sharded-slab v0.1.7
   Compiling tracing-log v0.2.0
   Compiling thread_local v1.1.10
   Compiling scoped-tls v1.0.1
   Compiling nu-ansi-term v0.50.3
   Compiling dotenvy v0.15.7
   Compiling serde_derive v1.0.229
   Compiling displaydoc v0.2.7
   Compiling tokio-macros v2.7.2
   Compiling thiserror-impl v2.0.20
   Compiling async-trait v0.1.92
   Compiling regex v1.13.1
   Compiling uuid v1.24.0
   Compiling parity-scale-codec-derive v3.7.5
   Compiling impl-trait-for-tuples v0.2.3
   Compiling thiserror-impl v1.0.69
   Compiling derive_more-impl v1.0.0
   Compiling zerofrom-derive v0.1.7
   Compiling scale-info-derive v2.11.6
   Compiling yoke-derive v0.8.2
   Compiling zerovec-derive v0.11.3
   Compiling futures-macro v0.3.33
   Compiling tracing-attributes v0.1.31
   Compiling auto_impl v1.3.0
   Compiling strum_macros v0.26.4
   Compiling num_enum_derive v0.7.6
   Compiling fixed-hash v0.8.0
   Compiling pin-project-internal v1.1.13
   Compiling Inflector v0.11.4
   Compiling hashlink v0.9.1
   Compiling futures-util v0.3.33
   Compiling zerofrom v0.1.8
   Compiling derive_more v1.0.0
   Compiling num_enum v0.7.6
   Compiling yoke v0.8.3
   Compiling matchers v0.2.0
   Compiling tracing v0.1.44
   Compiling pin-project v1.1.13
   Compiling simple_asn1 v0.6.4
   Compiling zerovec v0.11.6
   Compiling zerotrie v0.2.4
   Compiling tracing-futures v0.2.5
   Compiling strum v0.26.3
   Compiling tinystr v0.8.3
   Compiling potential_utf v0.1.5
   Compiling icu_collections v2.2.0
   Compiling icu_locale_core v2.2.0
   Compiling bytes v1.12.1
   Compiling impl-serde v0.4.0
   Compiling serde_spanned v0.6.9
   Compiling cargo-platform v0.1.9
   Compiling toml_datetime v0.6.11
   Compiling serde_urlencoded v0.7.1
   Compiling coins-core v0.8.7
   Compiling uuid v0.8.2
   Compiling tracing-serde v0.2.0
   Compiling cargo_metadata v0.18.1
   Compiling toml_edit v0.22.27
   Compiling tracing-subscriber v0.3.23
   Compiling eth-keystore v0.5.0
   Compiling rlp v0.5.2
   Compiling coins-bip32 v0.8.7
   Compiling impl-codec v0.6.0
   Compiling scale-info v2.11.6
   Compiling jsonwebtoken v8.3.0
   Compiling open-fastrlp-derive v0.1.1
   Compiling icu_provider v2.2.0
   Compiling http v0.2.12
   Compiling tokio v1.53.1
   Compiling http v1.5.0
   Compiling impl-rlp v0.3.0
   Compiling icu_properties v2.2.0
   Compiling icu_normalizer v2.2.0
   Compiling enr v0.10.0
   Compiling ethbloom v0.13.0
   Compiling primitive-types v0.12.2
   Compiling coins-bip39 v0.8.7
   Compiling futures-executor v0.3.33
   Compiling futures v0.3.33
   Compiling http-body v0.4.6
   Compiling headers-core v0.2.0
   Compiling headers v0.3.9
   Compiling toml v0.8.23
   Compiling idna_adapter v1.2.2
   Compiling idna v1.1.0
   Compiling ethereum-types v0.14.1
   Compiling url v2.5.8
   Compiling ethabi v18.0.0
   Compiling open-fastrlp v0.1.4
   Compiling tungstenite v0.21.0
   Compiling sct v0.7.1
   Compiling rustls-webpki v0.101.7
   Compiling ethers-core v2.0.14
   Compiling tokio-util v0.7.19
   Compiling tokio-tungstenite v0.21.0
   Compiling tokio-rustls v0.24.1
   Compiling tungstenite v0.20.1
   Compiling h2 v0.3.27
   Compiling tokio-tungstenite v0.20.1
   Compiling ethers-contract-abigen v2.0.14
   Compiling ethers-contract-derive v2.0.14
   Compiling ethers-signers v2.0.14
   Compiling ethers-addressbook v2.0.14
   Compiling hyper v0.14.32
   Compiling hyper-rustls v0.24.2
   Compiling warp v0.3.7
   Compiling reqwest v0.11.27
   Compiling ethers-providers v2.0.14
   Compiling ethers-etherscan v2.0.14
   Compiling ethers-contract v2.0.14
   Compiling ethers-middleware v2.0.14
   Compiling ethers v2.0.14
   Compiling rusqlite v0.31.0
   Compiling r2d2_sqlite v0.24.0
   Compiling sharklet v0.2.0 (/app)
error: mismatched closing delimiter: `)`

   --> src/executor.rs:117:134
    |
117 | ..._per_gas: f64) -> Result<ExecutionResult, ExecutorError> {
    |                                                             ^ unclosed delimiter
...
143 | ...s_u128() as f64 / 1e18 * gas_priceself, amount: U256) -> Result<U256, ExecutorError> {
    |                                                        ^ mismatched closing delimiter
error: could not compile `sharklet` (bin "sharklet") due to 1 previous error

Build Failed: build daemon returned an error < failed to solve: process "/bin/sh -c cargo build --release 2>&1 | tee /tmp/build.log;     test -f target/release/sharklet || (echo \"BUILD FAILED - dumping log\" && cat /tmp/build.log && exit 1)" did not complete successfully: exit code: 1 >
You reached the end of the range
2026-08-10 21:57


