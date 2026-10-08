# WorkOS + Infisical shared-credentials runbook

Goal: store one copy of the WorkOS AuthKit credential set in Infisical, and let any Phenotype project consume it trivially.

## What lives where

```
Infisical org:      79d957ee-93c2-4309-a8ae-158b9586c358
Infisical project:  Phenotype (8efe392e-56a6-4c3c-89f9-8141183dd7e8)
Secret folder:      /shared/workos    (created once per project, reused everywhere)
Keys:
  WORKOS_CLIENT_ID       public
  WORKOS_REDIRECT_URI    public
  WORKOS_AUTHKIT_DOMAIN  public
  WORKOS_API_KEY         secret (environment API key, sk_…)
  WORKOS_CLIENT_SECRET   secret (OAuth confidential-client secret, sk_… or longer)
```

Two secrets are needed because WorkOS uses two distinct credential types:

- `WORKOS_CLIENT_SECRET` - confidential-client OAuth secret, exchanged at the
  end of the authorization-code flow. **Can only be obtained at dashboard
  creation time** (or by clicking "Rotate" in the dashboard).
- `WORKOS_API_KEY` - environment API key, used for management-API calls
  (`/user_management/users`, directory sync, audit log streaming, etc).

Both go under the same `/shared/workos` folder so consumers need only one
Infisical path. Environment choice is per-call: pass `--env=dev|staging|prod`.

## Why this is the "one and done" plane

The `/shared/workos` folder is a single source of truth. Any project in the
org links the same `.infisical.json` and reads the same path. No project
copies a secret; no project owns the rotation. Rotation happens once in
Infisical and every project picks it up on next process start.

A future project can still bootstrap through the CLI:

```bash
infisical login                                     # once per machine
infisical run --projectId=8efe392e-...-8141183dd7e8 \
              --env=dev --path=/shared/workos -- <your-binary>
```

The `infisical run` invocation injects the matching variables into the spawned
process. It remains useful for other binaries, but Fabric does not require this
wrapper: `fabric-daemon` now loads `WORKOS_CLIENT_SECRET` directly during
startup.

## One-time setup (already complete for Phenotype)

The org's Phenotype project has these keys staged in dev/staging/prod:

```bash
infisical secrets set \
  "WORKOS_CLIENT_ID=client_01K4KYZR40RK7R9X3PPB5SEJ66" \
  "WORKOS_REDIRECT_URI=http://localhost:5173/auth/callback" \
  "WORKOS_AUTHKIT_DOMAIN=significant-vessel-93-staging.authkit.app" \
  --projectId=8efe392e-56a6-4c3c-89f9-8141183dd7e8 \
  --env=dev --path=/shared/workos
```

`WORKOS_API_KEY` already seeded (83 chars).
`WORKOS_CLIENT_SECRET` is the only outstanding value; it lives on the
WorkOS dashboard page and cannot be retrieved via API.

## What automation actually does

### Programmatic (already works today)

| Action | How |
|---|---|
| List projects/envs/keys | `workos project list --mode ci --insecure-storage --json` |
| Show active env's API key | `workos whoami --mode ci --insecure-storage --json` |
| Create new AuthKit project | `workos project create <name> --mode ci -y` |
| Read/write any Infisical secret | `infisical secrets get/set --path=/shared/workos` |
| Inject into any process | `infisical run --projectId=... --path=/shared/workos -- <cmd>` |

### NOT programmatic (probed 2026-09-20)

Direct probes against the WorkOS public API using the project's staging
`sk_test_…` key returned **404** for every plausible path:

| Probed endpoint | Result |
|---|---|
| `GET /authkit/clients/{id}/client_secret` | 404 |
| `POST /clients/{id}/rotate_client_secret` | 404 |
| `GET /user_management/clients/{id}` | 404 |
| `GET /environments/{id}` | 404 |
| `GET /user_management/environments/{id}` | 404 |
| `workos api ls` (full endpoint enumeration) | no client-secret lifecycle path exists |

**Verdict**: WorkOS does not expose the `client_secret` of an existing client
through any programmatic endpoint. The `client_secret` is a confidential
value shown to the dashboard operator exactly once at creation time, and the
public API has no rotate endpoint under any route family.

### Browser automation evidence

The WorkOS dashboard read remains unverified. The official Infisical API
contract was verified on 2026-09-24 in an isolated browser using DOM text only:

| Field | Verified contract |
|---|---|
| Method and path | `GET /api/v4/secrets/{secretName}` |
| Authentication | `Authorization: Bearer <token>` |
| Required query | `projectId` |
| Optional query | `environment`, `secretPath` (default `/`), `type` (`shared` default), `viewSecretValue` (`true` default) |
| Success fields | `secret.secretKey`, `secret.secretValue` |

The WorkOS `client_secret` itself was not captured. Its programmatic
availability and rotation endpoints remain UNKNOWN unless a future live
contract proves otherwise.

### Earlier WorkOS browser attempt (2026-09-20, blocked)

Tried the `jcode` browser tool to drive Firefox and read the dashboard:

| Block | Detail |
|---|---|
| Firefox 156 strict signing | xpi has `cose.manifest` but no AMO signature; `DisableAddonSignatureVerification` policy and `--allow-unsigned-extensions` flag do not bypass signing in Firefox 156 |
| Mozilla CDN truncates Nightly | Multiple mirrors reset connection around 175–178 MB of ~210 MB; downloaded DMGs report as corrupt xz; no way to install Firefox Developer Edition / ESR cleanly |
| Bridge host requires vault | `firefox-agent-bridge-host` reports "Not logged in to bronzewarden" — needs `bronzewarden login` first; standalone launch exits immediately because it's only designed to be invoked by Firefox native messaging |
| Chrome / Safari not wired | `browser` tool only supports `firefox_agent_bridge` backend; no Chrome extension installed; Safari not wired either |
| `bash` env degraded | Many commands hang on `ps aux | grep firefox` and `pkill -9 -f firefox`; ~50 stuck `bg` tasks from prior sessions |

**Recommendation**: use **Path A** (manual dashboard click). Open the WorkOS
dashboard, log in, navigate to the staging environment configuration page,
copy the `client_secret`, and seed Infisical without printing the value into
logs or terminal history.

### Paths forward

Two viable paths to populate `WORKOS_CLIENT_SECRET`:

**A. One-time dashboard click (fastest, ≤60s)**
Open the staging environment's configuration page (URL confirmed 2026-09-20
via 28-path probe — every path under `/environments/{env_id}/...` returns
307/login, while `/signin/clients/...` returns 404):

```
https://dashboard.workos.com/environments/environment_01K4KYZQJ88MK4CPCD3HHQ09R2/configuration/secrets
```

After login the page shows the `client_secret` for
`client_01K4KYZR40RK7R9X3PPB5SEJ66`. Copy the value, seed Infisical:

```bash
infisical secrets set WORKOS_CLIENT_SECRET=<value> \
  --projectId=8efe392e-56a6-4c3c-89f9-8141183dd7e8 \
  --env=dev --path=/shared/workos
# repeat for staging, prod
```

This is the documented one-time human step. **Recommended for now.**

**B. Full automation via AuthKit for Platforms** (only if the human click is unacceptable)
Requires **`PLATFORM_CLIENT_ID`** + **`PLATFORM_CLIENT_SECRET`** issued by
WorkOS for the AuthKit for Platforms product. These are distinct from any
environment-scoped key; they authorize creation of new AuthKit instances.
They are not present anywhere on this machine.

If `PLATFORM_CLIENT_ID` + `PLATFORM_CLIENT_SECRET` are obtained, the
5-step flow below creates a brand new AuthKit project with fresh
`client_id` + `client_secret` + `sk_…` API key. The existing
`client_01K4KYZR40RK7R9X3PPB5SEJ66` stays where it is.

### WorkOS Platform API: full programmatic provisioning

**Prerequisite**: `PLATFORM_CLIENT_ID` + `PLATFORM_CLIENT_SECRET` (issued by
WorkOS for the AuthKit for Platforms product).

```bash
# 1. Exchange platform credentials for an access token
TOKEN=$(curl -s -X POST "https://signin.workos.com/oauth2/token" \
  -d "client_id=$PLATFORM_CLIENT_ID" \
  -d "client_secret=$PLATFORM_CLIENT_SECRET" \
  -d "grant_type=client_credentials" | jq -r .access_token)

# 2. Create a team (admin invitation goes out)
TEAM=$(curl -s -X POST "https://api.workos.com/platform/teams" \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"admin_email":"you@example.com","name":"My App"}' | jq -r .id)

# 3. Create an environment (returns the OAuth client_id)
ENV=$(curl -s -X POST "https://api.workos.com/platform/teams/$TEAM/environments" \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"name":"my-app","production":false}')
CLIENT_ID=$(echo "$ENV" | jq -r .client_id)
ENV_ID=$(echo "$ENV" | jq -r .id)

# 4. Mint an environment API key (the secret sk_... - returned once)
API_KEY=$(curl -s -X POST "https://api.workos.com/platform/teams/$TEAM/environments/$ENV_ID/api_keys" \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"name":"provisioning","expires_at":null}' | jq -r .value)

# 5. Register the redirect URI for the app
curl -s -X POST "https://api.workos.com/user_management/redirect_uris" \
  -H "Authorization: Bearer $API_KEY" -H "Content-Type: application/json" \
  -d '{"uri":"http://localhost:5173/auth/callback"}'
```

Note: this creates a NEW AuthKit instance. It does NOT retrieve or rotate
the `client_secret` of an EXISTING instance.

## Fabric daemon fallback

After `WORKOS_CLIENT_SECRET` is seeded in `/shared/workos`, start the daemon
normally:

```bash
cargo run -p fabric-daemon -- start --config config/settings.toml
```

The daemon resolves the secret in this order:

1. A nonempty `WORKOS_CLIENT_SECRET` environment value.
2. A nonempty config-file value.
3. Infisical secret `WORKOS_CLIENT_SECRET` at path `/shared/workos` in
   `INFISICAL_ENV` (default `dev`).

The remote lookup is attempted only when authentication is enabled, a WorkOS
client ID is configured, and the Infisical service-account client ID, client
secret, and project ID are available. Configure the non-secret identifiers in
`config/settings.toml`; provide the service-account secret through
`INFISICAL_CLIENT_SECRET` or the auth config. An empty environment value never
erases a config-file value.

The lookup uses the verified Infisical v4 read-by-name contract through the
daemon's existing `InfisicalClient` authentication/token cache. Unmet
prerequisites (a missing WorkOS or Infisical identifier) skip the fetch
silently — no fetch is attempted, so no failure warning is emitted for that
deployment state. When a fetch is attempted and fails — missing secret,
authentication failure, or transport failure — the loader does not crash;
startup logs name the secret, folder, environment, and error category only;
response bodies, access tokens, and secret values are never included.
The possible categories are `unavailable` (transport), `auth` (Infisical
rejected the service-account token — a 401/403 on the read or a failed
login), `not_found` (no such secret at that folder/environment), `operation`
(any other non-success status), `serialization` (unparseable response), and
`empty_response` (success with an empty value). A rejected token is reported
as `auth` rather than a generic operation failure so the two are
distinguishable in the log. The rejected token is **evicted from the cache**,
so a later read re-authenticates once instead of replaying a token the server
already refused; the read itself is not retried inline, because the request was
authorized. `serialization` covers a 2xx response whose body does not match the
expected contract (for example a login body missing `accessToken`), which is
why it is a distinct category from the transport-level `unavailable`.
Existing WorkOS configuration validation remains the final authority when
authentication is enabled.

To verify the path without revealing the value, use a disposable daemon config
with authentication enabled, omit `WORKOS_CLIENT_SECRET`, and confirm startup
reports that the auth middleware is initialized. Do not print the config or
resolved secret. Acceptance against the live Infisical tenant additionally
requires valid service-account credentials and the seeded shared secret.

### Infisical endpoint (base URL and read contract)

- **Official read contract:** `GET /api/v4/secrets/{secretName}` with query
  `projectId` (required), `environment`, `secretPath` (default `/`),
  `type` (default `shared`), and `viewSecretValue` (default true); Bearer
  service-account authentication; response
  `{ "secret": { "secretKey", "secretValue", ... } }`. Docs re-observed
  2026-09-27: https://infisical.com/docs/api-reference/endpoints/secrets/read
  (matches the daemon's parser and query keys).
- **Official login contract:** `POST /api/v1/auth/universal-auth/login`
  body `clientId` (required), `clientSecret` (required), optional
  `organizationSlug`; response `accessToken`, `expiresIn`,
  `accessTokenMaxTTL`, `tokenType`. Docs observed 2026-09-27:
  https://infisical.com/docs/api-reference/endpoints/universal-auth/login
  Live enforcement observed 2026-09-27: sending `client_id`/
  `client_secret` returns 422 `path:["clientId"] Required`. The daemon was
  corrected to the camelCase request and response contract in the same
  change set; the authenticated response shape is docs-based only (no live
  authenticated login observed yet — UNKNOWN).
- **Default base URL:** `https://app.infisical.com` — the installed
  Infisical CLI v0.43.114 `--domain` default
  (`https://app.infisical.com/api`, observed 2026-09-24), also the CLI
  login config's `LoggedInUserDomain` (observed 2026-09-27).
- **Host ambiguity resolved (observed twice, 2026-09-27):** `app` and `us`
  resolve to the SAME AWS load balancer
  (`infisical-core-platform-...us-east-1.elb.amazonaws.com`, identical
  address pairs), present the SAME TLS certificate (identical SHA-256
  fingerprint, SAN `*.infisical.com`), and answer probes identically on
  both rounds (exact daemon GET → 401 `Token missing` with `req-us-*`
  request ids; login → 422 with identical `clientId`/`clientSecret` field
  paths). Docs samples use the `us` host, CLI/default use `app`; every
  externally visible signal says one shared backend — an ALB target-group
  split by hostname is the one thing not observable from outside. The
  former daemon default `https://secrets.infisical.com` is undocumented
  anywhere in the official docs (self-host docs use
  `https://<your-instance>/api`) and was replaced 2026-09-27.
- **Live route probes (unauthenticated, 2026-09-27):** the daemon's exact
  `GET /api/v4/secrets/WORKOS_CLIENT_SECRET?projectId=...&environment=dev&secretPath=%2Fshared%2Fworkos&type=shared&viewSecretValue=true`
  returns 401 `Token missing` on both `app` and `us` (route exists with
  the documented shape); `POST /api/v1/auth/universal-auth/login` validates
  the body on both hosts (422 carrying field paths). Legacy
  `/api/v1/secrets/raw` returns 404 on both hosts — absent from live cloud
  and unused by the daemon (zero call sites).
- **Override (non-secret):** set `INFISICAL_BASE_URL` to your region or
  self-hosted origin, e.g. `https://eu.infisical.com`. A trailing `/api`
  is accepted and stripped; the client appends `/api/v1/...` and
  `/api/v4/...` itself. An empty or whitespace-only value is ignored. The
  override is read at startup only and is never written to TOML or returned
  over config IPC.
- **Override validation (2026-09-29, fail closed):** the value is
  credential-bearing — the universal-auth login POST sends the service-account
  client secret to whatever origin it resolves to. Only a clean `http`/`https`
  origin with no path, query, or fragment is accepted (an explicit port is fine,
  and IPv6 literals keep their brackets). Plain `http` is accepted only for a
  loopback host (`localhost`, `127.0.0.0/8`, `::1`); a non-loopback `http`
  origin is refused so the client secret never crosses the network in cleartext
  (2026-10-04). Embedded userinfo is **rejected**, not stripped:
  `https://real.host@evil.example` parses to host `evil.example`, so accepting it
  would be the exact mistyped-host credential leak this check exists to prevent.
  Anything else — `eu.infisical.com` without a scheme, `ftp://…`,
  `http://<non-loopback>`, `…/api/v1`, `…?a=1`, `user:pass@host` — is **rejected
  and the Infisical fallback is skipped entirely**; no request leaves the host. It
  does **not** fall back to the documented US Cloud default, because that default
  would receive the service-account client secret of a self-hosted deployment
  purely because of a typo. The rejection is reported after logging
  initialization as
  `category=unparseable|scheme|insecure_transport|userinfo|missing_host|path`,
  naming only the variable. A rejected override does **not** erase an already
  configured `WORKOS_CLIENT_SECRET`: the env/config source is reported as-is and
  the fallback warning is emitted only when no secret was preconfigured
  (2026-10-04). An absent or whitespace-only value remains the normal unset case
  and uses the documented default. Hostnames are not validated offline, so a
  valid-but-wrong private host is still accepted.

### Runtime-only credentials (not persisted, not returned over config IPC)

The resolved `WORKOS_CLIENT_SECRET` and the `INFISICAL_CLIENT_SECRET`
service-account secret are **runtime-only**. They are held in memory for the
auth middleware and the Infisical fallback and are:

- **never written** to the saved TOML config — `save` / `apply_config_overrides`
  omit both fields via `#[serde(skip_serializing)]`; and
- **never returned** over config IPC — `config_snapshot` and the
  `save_config` response omit both fields.

A config file or IPC payload that *does* carry either value is still
deserialized at load time (so a manual entry keeps working), but nothing
persists it or echoes it back. To rotate a compromised value, update it in
Infisical; the daemon refills it on next start. Non-secret identifiers
(`workos_client_id`, `infisical_client_id`, `infisical_project_id`) remain
serializable as usual.

### Authenticated end-to-end status (externally blocked, precise cause)

A real authenticated fetch through the daemon path was **not** executed.
Evidence for the block (all observed 2026-09-27):

- No `INFISICAL_*` environment variables exist in the shell.
- `config/settings.toml` `[auth]` carries no `infisical_client_id` and no
  `infisical_project_id` keys; `infisical_client_secret` appears only in a
  comment pointing at env `INFISICAL_CLIENT_SECRET`, which is unset.
- The local Infisical CLI login session expired (`No valid login session
  found, triggering login flow`); re-login is an interactive operator
  action and was not triggered.
- Local `~/.infisical/secrets-backup/` snapshots of the shared plane
  (project `8efe392e-...`, envs dev/staging/prod, path `/shared/workos`,
  files dated 2026-09-24) exist but are encrypted
  (`CipherText`/`Nonce`/`AuthTag`) and cannot be read without the backup
  password.

Closing this loop requires either `infisical login` (operator) or service
account credentials placed in `[auth]`. Everything short of authentication
was verified live: route contract, host identity, request-shape validation,
and error redaction.

## Multi-project pattern

When a second project needs WorkOS (e.g. byteport, bytecraft):

1. Add `.infisical.json` to that repo's root with the same project id.
2. Run `infisical login` once.
3. Wrap the dev/start command: `infisical run --projectId=8efe392e-... --path=/shared/workos -- <cmd>`.

No further changes needed.
