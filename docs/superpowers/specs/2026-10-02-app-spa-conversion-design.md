# Design Specification: Porting `@cms/app` to Pure TanStack Router SPA for Rust Axum

**Date:** 2026-10-02  
**Status:** Approved  
**Target:** Convert `apps/app` from TanStack Start / Nitro SSR to a standalone Vite + TanStack Router SPA, completely eliminating `@cms/server`, `@cms/auth`, and `@cms/usage` workspace dependencies, and directly connecting to the Rust Axum (`cms-rs`) backend.

---

## 1. Background & Context

The original project (`nibleaf`) used a Node.js monorepo architecture:
- `apps/server`: Hono REST API + Better Auth mounted at `/api`.
- `apps/app`: TanStack Start application performing SSR with Nitro, invoking backend functions via typed Hono RPC (`@cms/server/rpc`) and client auth via Better Auth (`@cms/auth/client`).

The target project (`cms-rs`) ports all server logic to Rust (Axum, SQLx). Static analysis and integration testing confirmed that:
1. Axum serves endpoints matching the exact same `/api/...` path conventions, HTTP verbs, and payload shapes as the original server.
2. Axum directly serves client SPA assets through `crates/cms-sites/src/lib.rs` (`serve_spa_file`) from `dist/frontend` or `frontend/dist` with `index.html` fallback.
3. Node.js SSR is no longer required or desired for `apps/app`.

The goal is to cleanly convert `apps/app` in-place into a pure client-side SPA that builds directly into the directory expected by Rust Axum, maintaining 100% of the UI features, routes, and styling without regressions.

---

## 2. Architecture Comparison

### Before (TanStack Start / SSR):
```
Browser ──> Node Server (Nitro / TanStack Start / server.ts)
                 ├── Custom Domain Rewrite & Nonce Injection
                 ├── SSR Page Rendering & Loader Execution
                 ├── Better Auth (@cms/auth/server)
                 └── Hono RPC Client (@cms/server/rpc) ──> Hono Backend
```

### After (Pure Vite SPA + Rust Axum):
```
Browser ──> Axum Server (cms-rs binary)
                 ├── Host Resolution & Static File Server (dist/frontend/index.html)
                 └── /api/auth & /api/app & /api/public (Native Rust Handlers)
                          ▲
                          │ fetch with credentials: 'include'
                          ▼
            Vite Client SPA (apps/app)
                 ├── TanStack Router (Client-side route matching & prefetching)
                 ├── TanStack Query v5 (queryOptions & caching)
                 ├── Dynamic Proxy API Client (Zero-churn RPC syntax replacement)
                 └── Native Auth Client (Direct HTTP calls to Axum auth endpoints)
```

---

## 3. Workspace & Dependency Adjustments

### 3.1 Workspace Definition
Create `pnpm-workspace.yaml` in the root of `cms-rs`:
```yaml
packages:
  - 'packages/*'
  - 'apps/*'
```

### 3.2 Update `apps/app/package.json`
- **Remove legacy dependencies**:
  - `@cms/server`: Replaced by lightweight proxy API client.
  - `@cms/auth`: Replaced by native Axum auth client.
  - `@cms/usage`: Inlined 2 type definitions.
  - `@tanstack/react-start`: Not needed in client SPA.
  - `nitro`: Not needed in client SPA.
- **Retain / Ensure**:
  - `@tanstack/react-router`: Core routing.
  - `@tanstack/router-plugin`: Vite plugin for `routeTree.gen.ts` generation.
  - `@tanstack/react-query`: Server state management.
  - `@cms/design-system`, `@cms/i18n`, `@cms/shared`, `@cms/validators`: In-tree workspace packages.
  - `@tiptap/*`, `@tailwindcss/vite`, `@mdx-js/rollup`, `lucide-react`, `sonner`, `katex`.
- **Scripts**:
  - `"dev"`: `"vite dev --port 4310"`
  - `"build"`: `"vite build"`
  - `"typecheck"`: `"tsc --noEmit"`

---

## 4. Vite & SPA Build Pipeline

### 4.1 `apps/app/vite.config.ts`
Transform Vite config from TanStack Start to pure SPA:
- Remove `tanstackStart()` and `nitro()`.
- Add `tanstackRouterVite({ routesDirectory: './src/routes', generatedRouteTree: './src/routeTree.gen.ts' })`.
- Retain `paraglideVitePlugin`, `mdx()`, `tailwindcss()`, and `viteReact({ compiler: true })`.
- Configure `build.outDir`: `'../../dist/frontend'` with `emptyOutDir: true`.
- Configure `server.proxy`:
  ```ts
  server: {
    port: 4310,
    proxy: {
      '/api': {
        target: process.env.VITE_API_URL || 'http://localhost:4311',
        changeOrigin: true,
      },
    },
  }
  ```

### 4.2 HTML Entry Point (`apps/app/index.html`)
Create standard Vite HTML entry:
```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>CMS</title>
    <link rel="icon" href="/favicon.svg" type="image/svg+xml" />
    <link rel="icon" href="/favicon-32x32.png" type="image/png" sizes="32x32" />
    <link rel="apple-touch-icon" href="/apple-touch-icon.png" />
    <link rel="manifest" href="/site.webmanifest" />
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

### 4.3 Client Bootstrap (`src/main.tsx` & `src/router.tsx`)
- **`src/main.tsx`**:
  ```tsx
  import React from 'react';
  import ReactDOM from 'react-dom/client';
  import { RouterProvider } from '@tanstack/react-router';
  import { QueryClientProvider } from '@tanstack/react-query';
  import { getRouter, queryClient } from './router';
  import './styles.css';

  const router = getRouter();

  ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </React.StrictMode>
  );
  ```
- **`src/router.tsx`**:
  Initialize `queryClient`, pass to TanStack Router context, configure `defaultPreload: 'intent'`, and export typed router interface.

---

## 5. API Client Architecture (`src/services/api.ts`)

Replace `@cms/server/rpc` with a lightweight, recursive ES Proxy. This preserves 100% of existing call sites across all 50+ hooks:

```ts
import { getLocale, REQUEST_LOCALE_HEADER } from '@cms/i18n';

type HttpMethod = 'get' | 'post' | 'put' | 'patch' | 'delete';

interface RequestArgs {
  param?: Record<string, string | number>;
  query?: Record<string, string | number | boolean | undefined>;
  json?: unknown;
  init?: RequestInit;
}

function buildApiProxy(pathSegments: string[] = []): any {
  return new Proxy(() => {}, {
    get(_target, prop: string) {
      if (prop.startsWith('$')) {
        const method = prop.slice(1).toUpperCase();
        return async (args?: RequestArgs) => {
          let resolvedPath = pathSegments.join('/');
          if (args?.param) {
            for (const [key, val] of Object.entries(args.param)) {
              resolvedPath = resolvedPath
                .replace(`:${key}`, encodeURIComponent(String(val)))
                .replace(`$${key}`, encodeURIComponent(String(val)));
            }
          }

          const url = new URL(resolvedPath, window.location.origin);
          if (args?.query) {
            for (const [key, val] of Object.entries(args.query)) {
              if (val !== undefined) url.searchParams.set(key, String(val));
            }
          }

          const headers = new Headers(args?.init?.headers);
          headers.set(REQUEST_LOCALE_HEADER, getLocale());
          if (args?.json !== undefined) {
            headers.set('Content-Type', 'application/json');
          }

          return fetch(url.toString(), {
            ...args?.init,
            method,
            headers,
            credentials: 'include',
            body: args?.json !== undefined ? JSON.stringify(args.json) : undefined,
          });
        };
      }
      return buildApiProxy([...pathSegments, prop]);
    },
  });
}

const rootProxy = buildApiProxy(['/api']);
export const api = rootProxy;
```

---

## 6. Native Auth Client (`src/services/auth-client.ts`)

Replace Better Auth client with a direct interface to Rust Axum `/api/auth/*` endpoints:

1. **`useSession()`**: TanStack Query hook querying `/api/auth/get-session` with 5-minute stale time. Returns `{ data, isPending, error, refetch }`.
2. **`authClient.emailOtp`**:
   - `sendVerificationOtp({ email, type })`: `POST /api/auth/email-otp/send-verification-otp`
   - `signIn.emailOtp({ email, otp })`: `POST /api/auth/sign-in/email-otp`
3. **`signIn.social({ provider, callbackURL })`**:
   - Calls `POST /api/auth/sign-in/social` and redirects window to returned OAuth URL.
4. **`authClient.signOut()`**:
   - `POST /api/auth/sign-out`, invalidates query cache, redirects to `/sign-in`.
5. **Auxiliary methods**: `updateUser`, `organization.acceptInvitation`, `admin.stopImpersonating`.

---

## 7. React Query v5 Best-Pattern Architecture

1. **`queryOptions()` Factories**:
   All read operations are modeled via `queryOptions({ queryKey, queryFn, enabled, staleTime })`.
2. **Router Preloading**:
   Route loaders use `context.queryClient.ensureQueryData(queryOption)` for zero-waterfall, pre-fetched navigation on hover.
3. **Targeted Invalidation**:
   Mutations invalidate precise scopes using the existing `queryKeys` factory (e.g. `queryKeys.pages.allForProject(projectId)`).
4. **Error Handling**:
   `ApiResponseError` preserves HTTP status codes, error codes, and field validation messages.

---

## 8. Server Code Cleanup & Inlining

1. **Delete Obsolete Files**:
   - `src/server.ts` (1,142 lines SSR Node entry)
   - `src/server.test.ts`
   - `src/env.server.ts`
2. **Refactor `src/functions/`**:
   - `session.ts`: `getSessionFn()` uses client `fetch('/api/auth/get-session')`.
   - `site.ts`: Strip `createServerFn`, keep async methods calling `api.public.sites`.
   - `site-search.ts`: Strip `createServerFn`, keep async methods calling search and answer APIs.
   - `marketing.ts`: Strip `createServerFn`, export client `getGithubStarsFn`.
3. **Inline Missing Types**:
   - Inline `LimitState` and `UsageAvailability` into `src/components/settings/usage-tab.tsx`.
4. **Clean `src/lib/site-origin.ts`**:
   - Remove `createIsomorphicFn`, return `window.location.origin`.
5. **Clean `src/routes/__root.tsx`**:
   - Remove SSR `nonce` parameter.

---

## 9. Verification & Acceptance Criteria

1. **Typecheck**: `pnpm --filter @cms/app typecheck` passes with 0 errors.
2. **Build**: `pnpm --filter @cms/app build` produces static bundle in `dist/frontend` with `index.html` and assets.
3. **Serving from Rust**: Launching `cms-server` on port `4311` (or configured port) properly serves the SPA on `/` and resolves all `/api/...` calls without 404 or CORS issues.
