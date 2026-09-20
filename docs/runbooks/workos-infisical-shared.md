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

A future project bootstraps in two commands:

```bash
infisical login                                     # once per machine
infisical run --projectId=8efe392e-...-8141183dd7e8 \
              --env=dev --path=/shared/workos -- <your-binary>
```

The `infisical run` invocation injects `WORKOS_CLIENT_ID`,
`WORKOS_REDIRECT_URI`, `WORKOS_AUTHKIT_DOMAIN`, `WORKOS_API_KEY`,
`WORKOS_CLIENT_SECRET` into the spawned process. The binary reads them via
its existing `std::env::var("WORKOS_CLIENT_SECRET")` call - no code change.

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

### Programmatic via WorkOS Platform API (no CLI required)

AuthKit for Platforms exposes full programmatic provisioning. New project =
4 API calls + 1 OAuth token exchange:

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

### NOT programmatic (one-time dashboard click)

| Action | Why |
|---|---|
| Reveal an existing project's `client_secret` | WorkOS only returns it at creation time on the dashboard. There is no `GET` endpoint for it. |
| Rotate an existing project's `client_secret` | `POST /authkit/clients/{id}/rotate_client_secret` returns 404 - the endpoint does not exist in the current WorkOS public API. |

Both are solved in 10 seconds by clicking "Show secret" / "Rotate secret"
on `https://dashboard.workos.com/signin/clients/<client_id>/secrets`. The
automation floor is one human click per AuthKit instance per lifetime.

## Fabric-specific wiring (next step)

After `WORKOS_CLIENT_SECRET` is dropped into `/shared/workos`, the daemon's
existing `config::load_env_secrets` should fall back to Infisical when the
env var is absent, instead of failing:

```rust
fn workos_client_secret() -> String {
    std::env::var("WORKOS_CLIENT_SECRET")
        .ok()
        .or_else(|| infisical_fetch("/shared/workos", "WORKOS_CLIENT_SECRET"))
        .unwrap_or_default()
}
```

The `InfisicalClient` already exists at
`crates/fabric-daemon/src/auth/secrets.rs`. Adding the fallback is a 20-line
change. After that, `.env` becomes optional for everything except local
development convenience.

## Multi-project pattern

When a second project needs WorkOS (e.g. byteport, bytecraft):

1. Add `.infisical.json` to that repo's root with the same project id.
2. Run `infisical login` once.
3. Wrap the dev/start command: `infisical run --projectId=8efe392e-... --path=/shared/workos -- <cmd>`.

No further changes needed.
