# Porting `@cms/app` to Pure TanStack Router SPA Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert `apps/app` from TanStack Start / Nitro SSR into a standalone Vite + TanStack Router SPA, completely eliminating `@cms/server`, `@cms/auth`, and `@cms/usage` workspace dependencies, and wiring directly with the Rust Axum backend.

**Architecture:** A pure client-side SPA built with Vite and TanStack Router, employing TanStack Query v5 with `queryOptions` and a zero-churn recursive proxy API client for `/api/*` endpoints, coupled with a native auth client hitting Axum's `/api/auth/*` routes directly with credentials. Axum's `cms-sites` serves the built SPA directly from `dist/frontend`.

**Tech Stack:** React 19, TypeScript, Vite 8, `@tanstack/react-router`, `@tanstack/react-query`, `@tanstack/router-plugin`, `@tailwindcss/vite`, `@inlang/paraglide-js`, Rust Axum.

## Global Constraints
- Target bundle output MUST be `../../dist/frontend` relative to `apps/app` (resolving to `cms-rs/dist/frontend`).
- Zero reliance on Node.js runtime for serving or rendering (no Nitro, no SSR server functions).
- Retain all 55 existing TanStack Router route files in `src/routes/` and all UI components.
- Retain existing API call site signatures across `src/hooks/api/` via proxy-based client.
- Auth session cookies must remain `better-auth.session_token` with `credentials: 'include'`.

---

### Task 1: Workspace Definition & Dependency Pruning

**Files:**
- Create: `pnpm-workspace.yaml` (in `cms-rs` root)
- Modify: `apps/app/package.json`

**Interfaces:**
- Consumes: Existing packages in `packages/*`
- Produces: Resolved pnpm workspace linking `@cms/design-system`, `@cms/i18n`, `@cms/shared`, `@cms/validators`

- [ ] **Step 1: Create `pnpm-workspace.yaml` in root**

Create `d:\Workspace\Software\Cloned-Repos\cms-rs\cms-rs\pnpm-workspace.yaml`:
```yaml
packages:
  - 'packages/*'
  - 'apps/*'
```

- [ ] **Step 2: Update `apps/app/package.json` to prune obsolete dependencies**

In `d:\Workspace\Software\Cloned-Repos\cms-rs\cms-rs\apps\app\package.json`:
- Remove dependencies:
  - `"@cms/auth": "workspace:*"`
  - `"@cms/server": "workspace:*"`
  - `"@cms/usage": "workspace:*"`
  - `"@tanstack/react-start": "^1.168.49"`
  - `"nitro": "3.0.260522-beta"`
- Add devDependency:
  - `"@tanstack/router-plugin": "^1.170.32"`
- Update scripts:
  - `"build": "vite build"`
  - `"start": "vite preview"`

- [ ] **Step 3: Run `pnpm install` to update lockfile and link workspace packages**

Run: `pnpm install` in `d:\Workspace\Software\Cloned-Repos\cms-rs\cms-rs`  
Expected: Workspace packages resolve without error; obsolete packages unlinked.

---

### Task 2: Vite Configuration & SPA Entry Point

**Files:**
- Modify: `apps/app/vite.config.ts`
- Create: `apps/app/index.html`
- Create: `apps/app/src/main.tsx`
- Modify: `apps/app/src/router.tsx`

**Interfaces:**
- Consumes: `@tanstack/router-plugin/vite`, `@vitejs/plugin-react`
- Produces: Client entry HTML & SPA router with `queryClient` in router context

- [ ] **Step 1: Refactor `apps/app/vite.config.ts` to pure SPA**

Update `apps/app/vite.config.ts`:
- Import `tanstackRouterVite` from `@tanstack/router-plugin/vite`.
- Remove `tanstackStart` and `nitro` imports and plugin calls.
- Configure `tanstackRouterVite({ routesDirectory: './src/routes', generatedRouteTree: './src/routeTree.gen.ts' })`.
- Configure `build.outDir: '../../dist/frontend'` and `build.emptyOutDir: true`.
- Configure `server.proxy` to forward `/api` to `http://localhost:4311`.

- [ ] **Step 2: Create `apps/app/index.html`**

Create `d:\Workspace\Software\Cloned-Repos\cms-rs\cms-rs\apps\app\index.html`:
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

- [ ] **Step 3: Create `apps/app/src/main.tsx`**

Create `d:\Workspace\Software\Cloned-Repos\cms-rs\cms-rs\apps\app\src\main.tsx`:
```tsx
import React from 'react';
import ReactDOM from 'react-dom/client';
import { RouterProvider } from '@tanstack/react-router';
import { QueryClientProvider } from '@tanstack/react-query';
import { getRouter, queryClient } from './router';
import './styles.css';

const router = getRouter();

const rootElement = document.getElementById('root');
if (rootElement && !rootElement.innerHTML) {
  const root = ReactDOM.createRoot(rootElement);
  root.render(
    <React.StrictMode>
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </React.StrictMode>,
  );
}
```

- [ ] **Step 4: Update `apps/app/src/router.tsx` to include `queryClient` in context**

Update `apps/app/src/router.tsx`:
```tsx
import { createRouter as createTanStackRouter } from '@tanstack/react-router';
import { QueryClient } from '@tanstack/react-query';
import { ErrorPage } from '@/components/error-page';
import { NotFound } from '@/components/not-found';
import { PageLoader } from '@/components/page-loader';
import { hydratedCustomDomainProjectId, rewriteCustomDomainInput, rewriteCustomDomainOutput } from '@/lib/custom-domain-rewrite';
import { routeTree } from './routeTree.gen';

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false,
      retry: 1,
      staleTime: 30_000,
    },
  },
});

export function getRouter() {
  const customDomainProjectId = hydratedCustomDomainProjectId();
  const router = createTanStackRouter({
    routeTree,
    context: { queryClient },
    defaultNotFoundComponent: NotFound,
    defaultErrorComponent: ErrorPage,
    defaultPendingComponent: PageLoader,
    defaultPendingMs: 200,
    defaultPendingMinMs: 400,
    scrollRestoration: true,
    defaultPreload: 'intent',
    defaultPreloadStaleTime: 0,
    rewrite: customDomainProjectId
      ? {
          input: ({ url }) => rewriteCustomDomainInput(url, customDomainProjectId),
          output: ({ url }) => rewriteCustomDomainOutput(url, customDomainProjectId),
        }
      : undefined,
  });
  return router;
}

declare module '@tanstack/react-router' {
  interface Register {
    router: ReturnType<typeof getRouter>;
  }
}
```

---

### Task 3: Lightweight Proxy API Client

**Files:**
- Modify: `apps/app/src/services/api.ts`
- Create: `apps/app/src/services/api.test.ts`

**Interfaces:**
- Consumes: Browser `fetch`, `getLocale()`, `REQUEST_LOCALE_HEADER` from `@cms/i18n`
- Produces: `export const api` proxy supporting `api.app.projects[':id'].$get({ ... })`

- [ ] **Step 1: Write unit tests for API proxy client**

Create `apps/app/src/services/api.test.ts`:
```ts
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { api } from './api';

describe('API Proxy Client', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('builds correct URL and sends GET request', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({ data: { id: 'p1' } })));
    vi.stubGlobal('fetch', fetchMock);

    const res = await (api as any).app.projects[':id'].$get({
      param: { id: 'proj-123' },
      query: { lang: 'en' },
    });

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0];
    expect(url.toString()).toContain('/api/app/projects/proj-123?lang=en');
    expect(init.method).toBe('GET');
    expect(init.credentials).toBe('include');
  });

  it('builds correct POST request with json body', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({ data: { success: true } })));
    vi.stubGlobal('fetch', fetchMock);

    await (api as any).app.projects[':projectId'].pages.$post({
      param: { projectId: 'p1' },
      json: { title: 'New Page' },
    });

    const [url, init] = fetchMock.mock.calls[0];
    expect(url.toString()).toContain('/api/app/projects/p1/pages');
    expect(init.method).toBe('POST');
    expect(init.headers.get('Content-Type')).toBe('application/json');
    expect(JSON.parse(init.body)).toEqual({ title: 'New Page' });
  });
});
```

- [ ] **Step 2: Implement recursive ES Proxy in `apps/app/src/services/api.ts`**

Update `apps/app/src/services/api.ts`:
```ts
import { getLocale, REQUEST_LOCALE_HEADER } from '@cms/i18n';

interface RequestArgs {
  param?: Record<string, string | number>;
  query?: Record<string, string | number | boolean | undefined>;
  json?: unknown;
  init?: RequestInit;
}

function createApiProxy(segments: string[] = []): any {
  return new Proxy(() => {}, {
    get(_target, prop: string) {
      if (prop.startsWith('$')) {
        const method = prop.slice(1).toUpperCase();
        return async (args?: RequestArgs): Promise<Response> => {
          let path = segments.join('/');
          if (args?.param) {
            for (const [key, val] of Object.entries(args.param)) {
              path = path
                .replace(`:${key}`, encodeURIComponent(String(val)))
                .replace(`$${key}`, encodeURIComponent(String(val)));
            }
          }

          const baseOrigin = typeof window !== 'undefined' ? window.location.origin : 'http://localhost:4310';
          const url = new URL(path, baseOrigin);

          if (args?.query) {
            for (const [key, val] of Object.entries(args.query)) {
              if (val !== undefined && val !== null) {
                url.searchParams.set(key, String(val));
              }
            }
          }

          const headers = new Headers(args?.init?.headers);
          try {
            headers.set(REQUEST_LOCALE_HEADER, getLocale());
          } catch {
            // Ignore if i18n not initialized
          }

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
      return createApiProxy([...segments, prop]);
    },
  });
}

/** Typed API proxy client rooted at `/api`. Zero-churn replacement for Hono RPC. */
export const api = createApiProxy(['/api']);
```

- [ ] **Step 3: Run Vitest on `api.test.ts`**

Run: `npx vitest run src/services/api.test.ts` in `apps/app`  
Expected: PASS

---

### Task 4: Native Axum Auth Client

**Files:**
- Modify: `apps/app/src/services/auth-client.ts`
- Create: `apps/app/src/services/auth-client.test.ts`

**Interfaces:**
- Consumes: Browser `fetch`, TanStack Query `useQuery`, `queryClient`
- Produces: `useSession()`, `signIn`, `authClient` targeting Axum `/api/auth/*`

- [ ] **Step 1: Write unit tests for Auth Client**

Create `apps/app/src/services/auth-client.test.ts`:
```ts
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { authClient, signIn } from './auth-client';

describe('Auth Client', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('sendVerificationOtp calls Axum email-otp endpoint', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({ success: true })));
    vi.stubGlobal('fetch', fetchMock);

    const res = await authClient.emailOtp.sendVerificationOtp({
      email: 'test@example.com',
      type: 'sign-in',
    });

    expect(fetchMock).toHaveBeenCalledWith('/api/auth/email-otp/send-verification-otp', expect.objectContaining({
      method: 'POST',
      body: JSON.stringify({ email: 'test@example.com', type: 'sign-in' }),
      credentials: 'include',
    }));
    expect(res.data).toBeDefined();
  });

  it('signOut calls /api/auth/sign-out', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({ success: true })));
    vi.stubGlobal('fetch', fetchMock);

    await authClient.signOut();
    expect(fetchMock).toHaveBeenCalledWith('/api/auth/sign-out', expect.objectContaining({
      method: 'POST',
      credentials: 'include',
    }));
  });
});
```

- [ ] **Step 2: Implement native Auth Client in `apps/app/src/services/auth-client.ts`**

Update `apps/app/src/services/auth-client.ts`:
```ts
import { useQuery } from '@tanstack/react-query';
import { queryClient } from '@/router';

export interface User {
  id: string;
  email: string;
  name?: string;
  emailVerified?: boolean;
  createdAt?: string;
  updatedAt?: string;
}

export interface Session {
  id: string;
  userId: string;
  token?: string;
}

export interface SessionData {
  user: User;
  session: Session;
}

async function authFetch<T>(path: string, options?: RequestInit): Promise<{ data?: T; error?: { message?: string } }> {
  try {
    const headers = new Headers(options?.headers);
    if (options?.body && !headers.has('Content-Type')) {
      headers.set('Content-Type', 'application/json');
    }
    const res = await fetch(path, {
      ...options,
      headers,
      credentials: 'include',
    });
    if (!res.ok) {
      const err = await res.json().catch(() => ({}));
      return { error: { message: err?.message || err?.error?.message || `HTTP ${res.status}` } };
    }
    const data = await res.json().catch(() => ({}));
    return { data };
  } catch (e: any) {
    return { error: { message: e.message || 'Network error' } };
  }
}

export const sessionQueryKey = ['auth', 'session'] as const;

export function useSession() {
  const query = useQuery({
    queryKey: sessionQueryKey,
    queryFn: async (): Promise<SessionData | null> => {
      const res = await fetch('/api/auth/get-session', { credentials: 'include' });
      if (!res.ok) return null;
      const json = await res.json();
      return json?.user ? json : null;
    },
    staleTime: 5 * 60 * 1000,
  });

  return {
    data: query.data ?? null,
    isPending: query.isLoading,
    error: query.error,
    refetch: query.refetch,
  };
}

export const authClient = {
  emailOtp: {
    sendVerificationOtp: async (args: { email: string; type?: string }) =>
      authFetch('/api/auth/email-otp/send-verification-otp', {
        method: 'POST',
        body: JSON.stringify(args),
      }),
  },
  signIn: {
    emailOtp: async (args: { email: string; otp: string }) => {
      const result = await authFetch('/api/auth/sign-in/email-otp', {
        method: 'POST',
        body: JSON.stringify(args),
      });
      if (result.data) {
        queryClient.invalidateQueries({ queryKey: sessionQueryKey });
      }
      return result;
    },
  },
  social: async (args: { provider: string; callbackURL?: string }) => {
    const result = await authFetch<{ url?: string }>('/api/auth/sign-in/social', {
      method: 'POST',
      body: JSON.stringify(args),
    });
    if (result.data?.url) {
      window.location.href = result.data.url;
    }
    return result;
  },
  signOut: async () => {
    const result = await authFetch('/api/auth/sign-out', { method: 'POST' });
    queryClient.setQueryData(sessionQueryKey, null);
    queryClient.clear();
    return result;
  },
  updateUser: async (args: { name?: string; image?: string }) =>
    authFetch('/api/auth/update-user', { method: 'POST', body: JSON.stringify(args) }),
  organization: {
    acceptInvitation: async (args: { invitationId: string }) =>
      authFetch('/api/auth/organizations/accept-invitation', { method: 'POST', body: JSON.stringify(args) }),
  },
  admin: {
    stopImpersonating: async () =>
      authFetch('/api/auth/admin/stop-impersonating', { method: 'POST' }),
  },
};

export const signIn = {
  social: authClient.social,
  emailOtp: authClient.signIn.emailOtp,
};
```

- [ ] **Step 3: Run Vitest on `auth-client.test.ts`**

Run: `npx vitest run src/services/auth-client.test.ts` in `apps/app`  
Expected: PASS

---

### Task 5: Server Code Cleanup & Inlining

**Files:**
- Modify: `apps/app/src/functions/session.ts`
- Modify: `apps/app/src/functions/site.ts`
- Modify: `apps/app/src/functions/site-search.ts`
- Modify: `apps/app/src/functions/marketing.ts`
- Modify: `apps/app/src/lib/site-origin.ts`
- Modify: `apps/app/src/routes/__root.tsx`
- Modify: `apps/app/src/components/settings/usage-tab.tsx`
- Delete: `apps/app/src/server.ts`, `apps/app/src/server.test.ts`, `apps/app/src/env.server.ts`

- [ ] **Step 1: Simplify `apps/app/src/functions/session.ts`**

Replace with pure client fetch:
```ts
export const getSessionFn = async () => {
  try {
    const res = await fetch('/api/auth/get-session', { credentials: 'include' });
    if (!res.ok) return null;
    const json = await res.json();
    return json?.user ? json : null;
  } catch {
    return null;
  }
};
```

- [ ] **Step 2: Strip `createServerFn` from `apps/app/src/functions/site.ts`**

Convert methods to standard async functions:
```ts
import { getData } from '@/hooks/api/client-helpers';
import type { ChangelogEntry, SitePage, SiteShell } from '@/hooks/api/types';
import { api } from '@/services/api';

export const getSiteFn = async ({ data }: { data: { projectId: string; language?: string; version?: string } }) =>
  getData<SiteShell>(
    await api.public.sites[':id'].$get({
      param: { id: data.projectId },
      query: { ...(data.language ? { lang: data.language } : {}), ...(data.version ? { version: data.version } : {}) },
    }),
    'site',
  );

export const getSitePageFn = async ({ data }: { data: { projectId: string; path: string; language?: string; version?: string } }) =>
  getData<SitePage>(
    await api.public.sites[':id'].page.$get({
      param: { id: data.projectId },
      query: {
        path: data.path,
        ...(data.language ? { lang: data.language } : {}),
        ...(data.version ? { version: data.version } : {}),
      },
    }),
    'page',
  );

export const listSiteChangelogFn = async ({ data }: { data: { projectId: string } }) =>
  getData<ChangelogEntry[]>(await api.public.sites[':id'].changelog.$get({ param: { id: data.projectId } }), 'changelog');

export const getGitPreviewFn = async ({ data }: { data: { token: string } }) => {
  const response = await api.public.git.previews[':token'].$get({ param: { token: data.token } });
  return JSON.stringify(await getData(response, 'pull-request preview'));
};
```

- [ ] **Step 3: Strip `createServerFn` from `apps/app/src/functions/site-search.ts`**

Convert methods to standard async functions:
```ts
import { getData } from '@/hooks/api/client-helpers';
import type { SearchAnswer, SiteSearchHit } from '@/hooks/api/types';
import { api } from '@/services/api';

export const searchSiteFn = async ({ data }: { data: { projectId: string; query: string; language?: string; version?: string; limit?: number } }) => {
  const result = await getData<{ hits: SiteSearchHit[] }>(
    await api.public.sites[':id'].search.$get({
      param: { id: data.projectId },
      query: {
        q: data.query,
        ...(data.limit ? { limit: String(data.limit) } : {}),
        ...(data.language ? { lang: data.language } : {}),
        ...(data.version ? { version: data.version } : {}),
      },
    }),
    'search',
  );
  return result.hits;
};

export const answerSiteFn = async ({ data }: { data: { projectId: string; query: string; language?: string; version?: string } }) =>
  getData<SearchAnswer>(
    await api.public.sites[':id'].answer.$post({
      param: { id: data.projectId },
      json: {
        q: data.query,
        ...(data.language ? { lang: data.language } : {}),
        ...(data.version ? { version: data.version } : {}),
      },
    }),
    'answer',
  );
```

- [ ] **Step 4: Strip `createServerFn` from `apps/app/src/functions/marketing.ts`**

Export `getGithubStarsFn = async () => getGithubStars()`.

- [ ] **Step 5: Clean `apps/app/src/lib/site-origin.ts`**

Replace with:
```ts
export const customDomainOrigin = (): string | undefined => {
  return typeof window !== 'undefined' ? window.location.origin : undefined;
};
```

- [ ] **Step 6: Remove SSR nonce in `apps/app/src/routes/__root.tsx`**

In `apps/app/src/routes/__root.tsx`, line 50:
Remove `const nonce = useRouter().options.ssr?.nonce;` and any SSR-only nonce usages.

- [ ] **Step 7: Inline missing `@cms/usage` types in `apps/app/src/components/settings/usage-tab.tsx`**

In `apps/app/src/components/settings/usage-tab.tsx`, replace `import type { LimitState, UsageAvailability } from "@cms/usage";` with:
```ts
export type LimitState = 'ok' | 'approaching' | 'exceeded' | 'hard_stop';

export interface UsageAvailability {
  allowed: boolean;
  state: LimitState;
  remaining?: number;
  limit?: number;
}
```

- [ ] **Step 8: Delete obsolete SSR Node files**

Delete:
- `apps/app/src/server.ts`
- `apps/app/src/server.test.ts`
- `apps/app/src/env.server.ts`

---

### Task 6: Typecheck & Build Validation

**Files:**
- All files in `apps/app`

- [ ] **Step 1: Run TypeScript compiler check**

Run: `pnpm --filter @cms/app typecheck`  
Expected: Exit code 0, zero diagnostic errors.

- [ ] **Step 2: Run Vite production build**

Run: `pnpm --filter @cms/app build`  
Expected: Successful build into `dist/frontend/` with `index.html` and assets.

- [ ] **Step 3: Verify build directory contents**

Run: `Test-Path "d:\Workspace\Software\Cloned-Repos\cms-rs\cms-rs\dist\frontend\index.html"`  
Expected: True

---

### Task 7: End-to-End Rust Serving Verification

**Files:**
- `crates/cms-sites/src/lib.rs`

- [ ] **Step 1: Check Rust build**

Run: `cargo check -p cms-sites -p cms-server` in `cms-rs` root  
Expected: Success

- [ ] **Step 2: Verify `cms-server` finds the SPA**

Run `cms-server` or xtask to confirm `serve_spa_file` locates `dist/frontend/index.html` and responds with HTTP 200 and `text/html`.
