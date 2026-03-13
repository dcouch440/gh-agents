(base) davidcouch@Davids-Mac-Studio gh-agents % make run
cargo run
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.21s
     Running `target/debug/nexor`
2026-03-13T21:47:30.487Z  INFO nexor::commands::serve (serve.rs:29)                         nexor server starting...
2026-03-13T21:47:30.487Z DEBUG nexor::commands::serve (serve.rs:30)                         Debug logging enabled (verbosity: 0)
2026-03-13T21:47:30.488Z DEBUG nexor::config::global (mod.rs:23)                            Global config not found at "/Users/davidcouch/.config/nexor/config.toml", using defaults
2026-03-13T21:47:30.488Z DEBUG nexor::config::project (mod.rs:22)                           Project config not found at ".nexor/config.toml", using global only
2026-03-13T21:47:30.488Z  INFO nexor::db (mod.rs:28)                                        DB pool max_connections = 50
2026-03-13T21:47:30.556Z  INFO nexor::db (mod.rs:36)                                        Database connected to PostgreSQL
2026-03-13T21:47:30.562Z DEBUG sqlx::query (logger.rs:143)                                  summary="SELECT current_database()" db.statement="" rows_affected=1 rows_returned=1 elapsed=4.796292ms elapsed_secs=0.004796292
2026-03-13T21:47:30.564Z DEBUG sqlx::query (logger.rs:143)                                  summary="SELECT pg_advisory_lock($1)" db.statement="" rows_affected=1 rows_returned=1 elapsed=1.300292ms elapsed_secs=0.001300292
2026-03-13T21:47:30.565Z  INFO sqlx::postgres::notice (stream.rs:185)                       relation "_sqlx_migrations" already exists, skipping
2026-03-13T21:47:30.565Z DEBUG sqlx::query (logger.rs:143)                                  summary="CREATE TABLE IF NOT …" db.statement="\n\n\nCREATE TABLE IF NOT EXISTS _sqlx_migrations (\n    version BIGINT PRIMARY KEY,\n    description TEXT NOT NULL,\n    installed_on TIMESTAMPTZ NOT NULL DEFAULT now(),\n    success BOOLEAN NOT NULL,\n    checksum BYTEA NOT NULL,\n    execution_time BIGINT NOT NULL\n);\n                \n" rows_affected=0 rows_returned=0 elapsed=941.791µs elapsed_secs=0.000941791
2026-03-13T21:47:30.567Z DEBUG sqlx::query (logger.rs:143)                                  summary="SELECT version FROM _sqlx_migrations …" db.statement="\n\nSELECT version FROM _sqlx_migrations WHERE success = false ORDER BY version LIMIT 1\n" rows_affected=0 rows_returned=0 elapsed=2.4465ms elapsed_secs=0.0024465
2026-03-13T21:47:30.569Z DEBUG sqlx::query (logger.rs:143)                                  summary="SELECT version, checksum FROM …" db.statement="\n\nSELECT version, checksum FROM _sqlx_migrations ORDER BY version\n" rows_affected=61 rows_returned=61 elapsed=1.431708ms elapsed_secs=0.001431708
2026-03-13T21:47:30.570Z DEBUG sqlx::query (logger.rs:143)                                  summary="SELECT current_database()" db.statement="" rows_affected=1 rows_returned=1 elapsed=513.542µs elapsed_secs=0.000513542
2026-03-13T21:47:30.571Z DEBUG sqlx::query (logger.rs:143)                                  summary="SELECT pg_advisory_unlock($1)" db.statement="" rows_affected=1 rows_returned=1 elapsed=882.709µs elapsed_secs=0.000882709
2026-03-13T21:47:30.571Z  INFO nexor::db (mod.rs:44)                                        All migrations complete
2026-03-13T21:47:30.668Z  WARN nexor::server::state (mod.rs:280)                            JWT_SECRET not set — using random secret. Tokens will not survive restarts.
2026-03-13T21:47:30.671Z  INFO nexor::server::state (mod.rs:293)                            Loaded capability registry from /Users/davidcouch/Dev/gh-agents/config
2026-03-13T21:47:30.673Z  INFO nexor::server::state (mod.rs:319)                            Initialized Anthropic provider: claude-sonnet-4-20250514
2026-03-13T21:47:30.674Z DEBUG nexor::server::state (mod.rs:394)                            Ollama provider disabled (set NEXOR_OLLAMA_ENABLED=true to enable)
2026-03-13T21:47:30.674Z  INFO nexor::server::state (mod.rs:412)                            Initialized xAI provider: grok-4-0709 (https://api.x.ai) [web_search + x_search enabled]
2026-03-13T21:47:30.674Z  INFO nexor::server::state (mod.rs:444)                            Active provider profile: 'xai'
2026-03-13T21:47:30.674Z DEBUG aws_runtime::fs_util (fs_util.rs:31)                         loaded home directory src="HOME"
2026-03-13T21:47:30.675Z DEBUG aws_runtime::env_config::source (source.rs:173)              performing home directory substitution home="/Users/davidcouch" path="~/.aws/config"
2026-03-13T21:47:30.675Z DEBUG aws_runtime::env_config::source (source.rs:103)              home directory expanded before="~/.aws/config" after="/Users/davidcouch/.aws/config"
2026-03-13T21:47:30.675Z DEBUG aws_runtime::env_config::source (source.rs:113)              config file not found path=~/.aws/config
2026-03-13T21:47:30.675Z DEBUG aws_runtime::env_config::source (source.rs:150)              config file loaded path=Some("/Users/davidcouch/.aws/config") size=0
2026-03-13T21:47:30.675Z DEBUG aws_runtime::env_config::source (source.rs:173)              performing home directory substitution home="/Users/davidcouch" path="~/.aws/credentials"
2026-03-13T21:47:30.675Z DEBUG aws_runtime::env_config::source (source.rs:103)              home directory expanded before="~/.aws/credentials" after="/Users/davidcouch/.aws/credentials"
2026-03-13T21:47:30.675Z DEBUG aws_runtime::env_config::source (source.rs:113)              config file not found path=~/.aws/credentials
2026-03-13T21:47:30.675Z DEBUG aws_runtime::env_config::source (source.rs:150)              config file loaded path=Some("/Users/davidcouch/.aws/credentials") size=0
2026-03-13T21:47:30.677Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:204) timeout settings for this operation: TimeoutConfig { connect_timeout: Set(1s), read_timeout: Set(1s), operation_timeout: Set(30s), operation_attempt_timeout: Set(10s) }
2026-03-13T21:47:30.677Z DEBUG aws_smithy_runtime_api::client::interceptors::context (context.rs:263) entering 'serialization' phase
2026-03-13T21:47:30.677Z DEBUG aws_smithy_runtime_api::client::interceptors::context (context.rs:275) entering 'before transmit' phase
2026-03-13T21:47:30.677Z DEBUG aws_smithy_runtime::client::retries::strategy::standard (standard.rs:197) no client rate limiter configured, so no token is required for the initial request.
2026-03-13T21:47:30.677Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:269) retry strategy has OKed initial request
2026-03-13T21:47:30.677Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:310) beginning attempt #1
2026-03-13T21:47:30.678Z DEBUG aws_smithy_runtime::client::orchestrator::auth (auth.rs:318) using legacy auth and endpoint orchestration, resolving endpoint for auth scheme selection scheme_id=AuthSchemeId { scheme_id: "x-aws-ec2-metadata-token" } endpoint_params=EndpointResolverParams { inner: TypeErasedBox[!Clone]:(), property: {} }
2026-03-13T21:47:30.678Z DEBUG aws_config::imds::client::token (token.rs:216)               IMDS token cache miss
2026-03-13T21:47:30.680Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:204) timeout settings for this operation: TimeoutConfig { connect_timeout: Set(1s), read_timeout: Set(1s), operation_timeout: Set(30s), operation_attempt_timeout: Set(10s) }
2026-03-13T21:47:30.681Z DEBUG aws_smithy_runtime_api::client::interceptors::context (context.rs:263) entering 'serialization' phase
2026-03-13T21:47:30.681Z DEBUG aws_smithy_runtime_api::client::interceptors::context (context.rs:275) entering 'before transmit' phase
2026-03-13T21:47:30.681Z DEBUG aws_smithy_runtime::client::retries::strategy::standard (standard.rs:197) no client rate limiter configured, so no token is required for the initial request.
2026-03-13T21:47:30.681Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:269) retry strategy has OKed initial request
2026-03-13T21:47:30.681Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:310) beginning attempt #1
2026-03-13T21:47:30.682Z DEBUG aws_smithy_runtime::client::orchestrator::auth (auth.rs:318) using legacy auth and endpoint orchestration, resolving endpoint for auth scheme selection scheme_id=AuthSchemeId { scheme_id: "noAuth" } endpoint_params=EndpointResolverParams { inner: TypeErasedBox[!Clone]:(), property: {} }
2026-03-13T21:47:30.682Z DEBUG aws_smithy_runtime::client::orchestrator::endpoints (endpoints.rs:104) will apply endpoint Endpoint { url: "http://169.254.169.254/", headers: {}, properties: {} } endpoint_prefix=None
2026-03-13T21:47:30.682Z DEBUG aws_smithy_runtime_api::client::interceptors::context (context.rs:296) entering 'transmit' phase
2026-03-13T21:47:30.850Z DEBUG aws_smithy_http_client::client (client.rs:703)               new connector created in 168.248ms
2026-03-13T21:47:30.851Z DEBUG hyper_util::client::legacy::connect::http (http.rs:768)      connecting to 169.254.169.254:80
2026-03-13T21:47:31.853Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:399) encountered orchestrator error; halting
2026-03-13T21:47:31.853Z DEBUG aws_smithy_runtime::client::retries::strategy::standard (standard.rs:240) attempt #1 classified as NoActionIndicated, not retrying
2026-03-13T21:47:31.853Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:337) a retry is either unnecessary or not possible, exiting attempt loop
2026-03-13T21:47:31.853Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:359) encountered orchestrator error; halting
2026-03-13T21:47:31.854Z DEBUG aws_smithy_runtime::client::retries::strategy::standard (standard.rs:240) attempt #1 classified as NoActionIndicated, not retrying
2026-03-13T21:47:31.854Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:337) a retry is either unnecessary or not possible, exiting attempt loop
2026-03-13T21:47:31.854Z  WARN aws_config::imds::region (region.rs:66)                      failed to load region from IMDS err=failed to load IMDS session token: dispatch failure: timeout: client error (Connect): HTTP connect timeout occurred after 1s: timed out (FailedToLoadToken(FailedToLoadToken { source: DispatchFailure(DispatchFailure { source: ConnectorError { kind: Timeout, source: hyper_util::client::legacy::Error(Connect, HttpTimeoutError { kind: "HTTP connect", duration: 1s }), connection: Unknown } }) }))
2026-03-13T21:47:31.855Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:204) timeout settings for this operation: TimeoutConfig { connect_timeout: Set(1s), read_timeout: Set(1s), operation_timeout: Set(30s), operation_attempt_timeout: Set(10s) }
2026-03-13T21:47:31.855Z DEBUG aws_smithy_runtime_api::client::interceptors::context (context.rs:263) entering 'serialization' phase
2026-03-13T21:47:31.855Z DEBUG aws_smithy_runtime_api::client::interceptors::context (context.rs:275) entering 'before transmit' phase
2026-03-13T21:47:31.855Z DEBUG aws_smithy_runtime::client::retries::strategy::standard (standard.rs:197) no client rate limiter configured, so no token is required for the initial request.
2026-03-13T21:47:31.855Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:269) retry strategy has OKed initial request
2026-03-13T21:47:31.855Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:310) beginning attempt #1
2026-03-13T21:47:31.855Z DEBUG aws_smithy_runtime::client::orchestrator::auth (auth.rs:318) using legacy auth and endpoint orchestration, resolving endpoint for auth scheme selection scheme_id=AuthSchemeId { scheme_id: "x-aws-ec2-metadata-token" } endpoint_params=EndpointResolverParams { inner: TypeErasedBox[!Clone]:(), property: {} }
2026-03-13T21:47:31.855Z DEBUG aws_config::imds::client::token (token.rs:216)               IMDS token cache miss
2026-03-13T21:47:31.856Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:204) timeout settings for this operation: TimeoutConfig { connect_timeout: Set(1s), read_timeout: Set(1s), operation_timeout: Set(30s), operation_attempt_timeout: Set(10s) }
2026-03-13T21:47:31.856Z DEBUG aws_smithy_runtime_api::client::interceptors::context (context.rs:263) entering 'serialization' phase
2026-03-13T21:47:31.856Z DEBUG aws_smithy_runtime_api::client::interceptors::context (context.rs:275) entering 'before transmit' phase
2026-03-13T21:47:31.856Z DEBUG aws_smithy_runtime::client::retries::strategy::standard (standard.rs:197) no client rate limiter configured, so no token is required for the initial request.
2026-03-13T21:47:31.856Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:269) retry strategy has OKed initial request
2026-03-13T21:47:31.856Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:310) beginning attempt #1
2026-03-13T21:47:31.856Z DEBUG aws_smithy_runtime::client::orchestrator::auth (auth.rs:318) using legacy auth and endpoint orchestration, resolving endpoint for auth scheme selection scheme_id=AuthSchemeId { scheme_id: "noAuth" } endpoint_params=EndpointResolverParams { inner: TypeErasedBox[!Clone]:(), property: {} }
2026-03-13T21:47:31.856Z DEBUG aws_smithy_runtime::client::orchestrator::endpoints (endpoints.rs:104) will apply endpoint Endpoint { url: "http://169.254.169.254/", headers: {}, properties: {} } endpoint_prefix=None
2026-03-13T21:47:31.856Z DEBUG aws_smithy_runtime_api::client::interceptors::context (context.rs:296) entering 'transmit' phase
2026-03-13T21:47:31.857Z DEBUG aws_smithy_http_client::client (client.rs:703)               new connector created in 895µs
2026-03-13T21:47:31.857Z DEBUG hyper_util::client::legacy::connect::http (http.rs:768)      connecting to 169.254.169.254:80
2026-03-13T21:47:31.858Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:399) encountered orchestrator error; halting
2026-03-13T21:47:31.858Z DEBUG aws_smithy_runtime::client::retries::strategy::standard (standard.rs:240) attempt #1 classified as NoActionIndicated, not retrying
2026-03-13T21:47:31.858Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:337) a retry is either unnecessary or not possible, exiting attempt loop
2026-03-13T21:47:31.858Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:359) encountered orchestrator error; halting
2026-03-13T21:47:31.858Z DEBUG aws_smithy_runtime::client::retries::strategy::standard (standard.rs:240) attempt #1 classified as NoActionIndicated, not retrying
2026-03-13T21:47:31.858Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:337) a retry is either unnecessary or not possible, exiting attempt loop
2026-03-13T21:47:31.859Z  WARN aws_config::imds::region (region.rs:66)                      failed to load region from IMDS err=failed to load IMDS session token: dispatch failure: io error: client error (Connect): tcp connect error: Host is down (os error 64) (FailedToLoadToken(FailedToLoadToken { source: DispatchFailure(DispatchFailure { source: ConnectorError { kind: Io, source: hyper_util::client::legacy::Error(Connect, ConnectError("tcp connect error", 169.254.169.254:80, Os { code: 64, kind: Uncategorized, message: "Host is down" })), connection: Unknown } }) }))
2026-03-13T21:47:31.860Z DEBUG aws_sdk_sts::endpoint_lib (endpoint_lib.rs:12)               loading default partitions
2026-03-13T21:47:31.862Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:204) timeout settings for this operation: TimeoutConfig { connect_timeout: Set(1s), read_timeout: Set(1s), operation_timeout: Set(30s), operation_attempt_timeout: Set(10s) }
2026-03-13T21:47:31.862Z DEBUG aws_smithy_runtime_api::client::interceptors::context (context.rs:263) entering 'serialization' phase
2026-03-13T21:47:31.862Z DEBUG aws_smithy_runtime_api::client::interceptors::context (context.rs:275) entering 'before transmit' phase
2026-03-13T21:47:31.862Z DEBUG aws_smithy_runtime::client::retries::strategy::standard (standard.rs:197) no client rate limiter configured, so no token is required for the initial request.
2026-03-13T21:47:31.862Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:269) retry strategy has OKed initial request
2026-03-13T21:47:31.862Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:310) beginning attempt #1
2026-03-13T21:47:31.862Z DEBUG aws_smithy_runtime::client::orchestrator::auth (auth.rs:318) using legacy auth and endpoint orchestration, resolving endpoint for auth scheme selection scheme_id=AuthSchemeId { scheme_id: "x-aws-ec2-metadata-token" } endpoint_params=EndpointResolverParams { inner: TypeErasedBox[!Clone]:(), property: {} }
2026-03-13T21:47:31.862Z DEBUG aws_config::imds::client::token (token.rs:216)               IMDS token cache miss
2026-03-13T21:47:31.863Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:204) timeout settings for this operation: TimeoutConfig { connect_timeout: Set(1s), read_timeout: Set(1s), operation_timeout: Set(30s), operation_attempt_timeout: Set(10s) }
2026-03-13T21:47:31.863Z DEBUG aws_smithy_runtime_api::client::interceptors::context (context.rs:263) entering 'serialization' phase
2026-03-13T21:47:31.863Z DEBUG aws_smithy_runtime_api::client::interceptors::context (context.rs:275) entering 'before transmit' phase
2026-03-13T21:47:31.863Z DEBUG aws_smithy_runtime::client::retries::strategy::standard (standard.rs:197) no client rate limiter configured, so no token is required for the initial request.
2026-03-13T21:47:31.863Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:269) retry strategy has OKed initial request
2026-03-13T21:47:31.863Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:310) beginning attempt #1
2026-03-13T21:47:31.863Z DEBUG aws_smithy_runtime::client::orchestrator::auth (auth.rs:318) using legacy auth and endpoint orchestration, resolving endpoint for auth scheme selection scheme_id=AuthSchemeId { scheme_id: "noAuth" } endpoint_params=EndpointResolverParams { inner: TypeErasedBox[!Clone]:(), property: {} }
2026-03-13T21:47:31.863Z DEBUG aws_smithy_runtime::client::orchestrator::endpoints (endpoints.rs:104) will apply endpoint Endpoint { url: "http://169.254.169.254/", headers: {}, properties: {} } endpoint_prefix=None
2026-03-13T21:47:31.863Z DEBUG aws_smithy_runtime_api::client::interceptors::context (context.rs:296) entering 'transmit' phase
2026-03-13T21:47:31.864Z DEBUG aws_smithy_http_client::client (client.rs:703)               new connector created in 755µs
2026-03-13T21:47:31.864Z DEBUG hyper_util::client::legacy::connect::http (http.rs:768)      connecting to 169.254.169.254:80
2026-03-13T21:47:31.865Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:399) encountered orchestrator error; halting
2026-03-13T21:47:31.865Z DEBUG aws_smithy_runtime::client::retries::strategy::standard (standard.rs:240) attempt #1 classified as NoActionIndicated, not retrying
2026-03-13T21:47:31.865Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:337) a retry is either unnecessary or not possible, exiting attempt loop
2026-03-13T21:47:31.865Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:359) encountered orchestrator error; halting
2026-03-13T21:47:31.865Z DEBUG aws_smithy_runtime::client::retries::strategy::standard (standard.rs:240) attempt #1 classified as NoActionIndicated, not retrying
2026-03-13T21:47:31.865Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:337) a retry is either unnecessary or not possible, exiting attempt loop
2026-03-13T21:47:31.865Z  WARN aws_config::imds::region (region.rs:66)                      failed to load region from IMDS err=failed to load IMDS session token: dispatch failure: io error: client error (Connect): tcp connect error: Host is down (os error 64) (FailedToLoadToken(FailedToLoadToken { source: DispatchFailure(DispatchFailure { source: ConnectorError { kind: Io, source: hyper_util::client::legacy::Error(Connect, ConnectError("tcp connect error", 169.254.169.254:80, Os { code: 64, kind: Uncategorized, message: "Host is down" })), connection: Unknown } }) }))
2026-03-13T21:47:31.866Z DEBUG aws_sdk_s3::endpoint_lib (endpoint_lib.rs:12)                loading default partitions
2026-03-13T21:47:31.868Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:204) timeout settings for this operation: TimeoutConfig { connect_timeout: Set(3.1s), read_timeout: Unset, operation_timeout: Unset, operation_attempt_timeout: Unset }
2026-03-13T21:47:31.868Z DEBUG aws_smithy_runtime_api::client::interceptors::context (context.rs:263) entering 'serialization' phase
2026-03-13T21:47:31.868Z DEBUG aws_smithy_runtime_api::client::interceptors::context (context.rs:275) entering 'before transmit' phase
2026-03-13T21:47:31.868Z DEBUG aws_smithy_runtime::client::retries::strategy::standard (standard.rs:197) no client rate limiter configured, so no token is required for the initial request.
2026-03-13T21:47:31.868Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:269) retry strategy has OKed initial request
2026-03-13T21:47:31.869Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:310) beginning attempt #1
2026-03-13T21:47:31.869Z DEBUG aws_sdk_s3::endpoint_auth (endpoint_auth.rs:26)              resolving endpoint for auth scheme selection endpoint_params=EndpointResolverParams { inner: TypeErasedBox[!Clone]:Params { bucket: Some("nexor-system-store"), region: Some("us-east-1"), use_fips: false, use_dual_stack: false, endpoint: Some("http://localhost:9000"), force_path_style: true, accelerate: false, use_global_endpoint: false, use_object_lambda_endpoint: None, key: None, prefix: None, copy_source: None, disable_access_points: None, disable_multi_region_access_points: false, use_arn_region: None, use_s3_express_control_endpoint: None, disable_s3_express_session_auth: None }, property: {} }
2026-03-13T21:47:31.869Z DEBUG aws_config::meta::credentials::chain (chain.rs:98)           loaded credentials provider=Environment
2026-03-13T21:47:31.869Z DEBUG aws_smithy_runtime::client::identity::cache::lazy (lazy.rs:357) identity cache miss occurred; added new identity (took 252µs) new_expiration=2026-03-13T22:02:31.869592Z valid_for=899.999724s partition=IdentityCachePartition(9)
2026-03-13T21:47:31.870Z DEBUG aws_smithy_runtime::client::identity::cache::lazy (lazy.rs:372) loaded identity
2026-03-13T21:47:31.870Z DEBUG aws_smithy_runtime::client::orchestrator::endpoints (endpoints.rs:88) resolving endpoint endpoint_params=EndpointResolverParams { inner: TypeErasedBox[!Clone]:Params { bucket: Some("nexor-system-store"), region: Some("us-east-1"), use_fips: false, use_dual_stack: false, endpoint: Some("http://localhost:9000"), force_path_style: true, accelerate: false, use_global_endpoint: false, use_object_lambda_endpoint: None, key: None, prefix: None, copy_source: None, disable_access_points: None, disable_multi_region_access_points: false, use_arn_region: None, use_s3_express_control_endpoint: None, disable_s3_express_session_auth: None }, property: {TypeId(0x2989b24d9aafcf49351773d5f0b9a0ff): TypeErasedBox[!Clone]:Identity { data: Credentials { provider_name: "EnvironmentVariable", access_key_id: "minioadmin", secret_access_key: "** redacted **", expires_after: "never", property_0: TypeErasedBox[Clone]:[CredentialsEnvVars] }, expiration: None, property_0: TypeErasedBox[!Clone]:FrozenLayer(Layer { name: "IdentityResolutionFeatureIdTracking", items: [TypeErasedBox[!Clone]:Set([CredentialsEnvVars])] }) }} }
2026-03-13T21:47:31.870Z DEBUG aws_smithy_runtime::client::orchestrator::endpoints (endpoints.rs:104) will apply endpoint Endpoint { url: "http://localhost:9000/nexor-system-store", headers: {}, properties: {"authSchemes": Array([Object({"name": String("sigv4"), "signingRegion": String("us-east-1"), "disableDoubleEncoding": Bool(true), "signingName": String("s3")})])} } endpoint_prefix=None
2026-03-13T21:47:31.871Z DEBUG aws_smithy_runtime_api::client::interceptors::context (context.rs:296) entering 'transmit' phase
2026-03-13T21:47:31.872Z DEBUG aws_smithy_http_client::client (client.rs:703)               new connector created in 696µs
2026-03-13T21:47:31.873Z DEBUG hyper_util::client::legacy::connect::http (http.rs:768)      connecting to [::1]:9000
2026-03-13T21:47:31.873Z DEBUG hyper_util::client::legacy::connect::http (http.rs:771)      connected to [::1]:9000
2026-03-13T21:47:31.880Z DEBUG hyper_util::client::legacy::pool (pool.rs:402)               pooling idle connection for ("http", localhost:9000)
2026-03-13T21:47:31.881Z DEBUG aws_smithy_runtime_api::client::interceptors::context (context.rs:308) entering 'before deserialization' phase
2026-03-13T21:47:31.881Z DEBUG aws_smithy_runtime_api::client::interceptors::context (context.rs:328) entering 'deserialization' phase
2026-03-13T21:47:31.881Z DEBUG aws_sdk_s3::operation::head_bucket (head_bucket.rs:160)      extended_request_id=Some("dd9025bab4ad464b049177c95eb6ebf374d3b3fd1af9251148b658df7ac2e3e8")
2026-03-13T21:47:31.881Z DEBUG aws_sdk_s3::operation::head_bucket (head_bucket.rs:164)      request_id=Some("189C84E03B3F41F0")
2026-03-13T21:47:31.881Z DEBUG aws_smithy_runtime_api::client::interceptors::context (context.rs:340) entering 'after deserialization' phase
2026-03-13T21:47:31.882Z DEBUG aws_smithy_runtime::client::retries::strategy::standard (standard.rs:240) attempt #1 classified as NoActionIndicated, not retrying
2026-03-13T21:47:31.882Z DEBUG aws_smithy_runtime::client::orchestrator (orchestrator.rs:337) a retry is either unnecessary or not possible, exiting attempt loop
2026-03-13T21:47:31.882Z  INFO nexor::server::state (mod.rs:458)                            Initialized S3 backend: bucket=nexor-system-store, endpoint=http://localhost:9000
2026-03-13T21:47:31.882Z  WARN nexor::server (mod.rs:628)                                   CORS_ORIGINS not set — allowing all origins (dev mode). Set CORS_ORIGINS for production.
2026-03-13T21:47:31.883Z  INFO nexor::server::executors::chat (mod.rs:26)                   Chat consumer started with provider 'xai', model: grok-4-0709
2026-03-13T21:47:31.883Z  INFO nexor::server (mod.rs:119)                                   Rate limiting disabled (NEXOR_SKIP_RATE_LIMIT=1)
2026-03-13T21:47:31.894Z  INFO nexor::server (mod.rs:84)                                    Server listening on http://0.0.0.0:3000
2026-03-13T21:47:34.848Z DEBUG tower_http::trace::on_request (on_request.rs:80)             started processing request
2026-03-13T21:47:34.850Z  INFO nexor::server (mod.rs:712)                                   GET /ws 401 1ms  req=acf978ae
2026-03-13T21:47:34.850Z DEBUG tower_http::trace::on_response (on_response.rs:114)          finished processing request latency=2 ms status=401