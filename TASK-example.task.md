@NAV
id    | type | path                              | about                                          | tokens | load
AUTH  | MRM  | docs/auth-flow.mermaid             | OAuth2 + session management sequence diagram    | 480    | task:auth
DB    | MRM  | docs/data-model.mermaid            | Entity relationships: users, sessions, tokens   | 640    | task:auth
D001  | DEC  | decisions/001-use-tauri.md          | Why Tauri over Electron: perf + binary size     | 200    | reference
SRC   | DIR  | src/auth/                           | Auth module: login, register, session, middleware| 1800   | task:auth
TEST  | DIR  | tests/auth/                         | Auth test suite: unit + integration             | 900    | task:auth
CFG   | CFG  | Cargo.toml                          | Dependencies and build config                   | 180    | task:auth

@CTX
status: Auth module scaffolded with basic session handling. OAuth2 not yet implemented.
decisions: Using JWT for session tokens (see D001). Refresh tokens stored in httpOnly cookies.
constraints: Must support Google and GitHub OAuth2 providers. No session storage in database.
existing_code: src/auth/mod.rs has basic password auth. Need to extend, not replace.
architecture: See AUTH mermaid for the full flow. DB mermaid shows the user/session tables.

---
<!-- Total resource cost: ~4,200 tokens -->

# Task: Implement OAuth2 Login Flow

## Context

The auth module currently supports basic password authentication. We need to add OAuth2 support for Google and GitHub providers. The session management infrastructure (JWT + refresh tokens) is already in place.

## What to Build

1. **OAuth2 provider abstraction** (`src/auth/oauth.rs`)
   - Trait `OAuthProvider` with methods: `authorize_url()`, `exchange_code()`, `fetch_user_info()`
   - Implementations for Google and GitHub
   - Config struct that reads client_id/secret from environment variables

2. **OAuth2 route handlers** (`src/auth/routes.rs`)
   - `GET /auth/oauth/:provider` — redirects to provider's authorization URL
   - `GET /auth/oauth/:provider/callback` — exchanges code for token, creates/links user, issues JWT
   - Error handling for denied access, invalid state, token exchange failures

3. **User linking** (`src/auth/users.rs`)
   - If an OAuth user's email matches an existing password-auth user, link the accounts
   - Store provider + provider_user_id in a new `oauth_connections` table
   - One user can have multiple OAuth connections

## Patterns to Follow

- Match the existing code style in `src/auth/mod.rs`
- Use the existing `create_session()` function after OAuth login
- Error types should extend the existing `AuthError` enum
- All new routes go through the existing auth middleware

## Tests to Write

- Unit tests for each provider's URL generation and code exchange (mock HTTP)
- Integration test for the full callback flow (mock provider responses)
- Test account linking: new user, existing user with same email, duplicate connection
- Test error cases: invalid code, expired state, provider down

## Definition of Done

- All existing auth tests still pass
- New OAuth tests pass
- `cargo clippy` clean
- No unwrap() in production code paths
