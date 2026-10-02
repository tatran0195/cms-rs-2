# Frontend Migration Analysis - Original CMS Repository

## Date: 2026-08-27

## 🎯 Objective

Analyze the original CMS repository to understand its frontend architecture and create a migration plan to work with our new Rust backend.

---

## 📊 Original Repository Structure

```
cms-original/
├── apps/
│   ├── app/                    # Frontend application (Vite + @tanstack/react-start)
│   │   ├── src/
│   │   │   ├── components/    # React components
│   │   │   ├── functions/    # Server functions (SSR)
│   │   │   ├── layouts/      # Layout components
│   │   │   ├── lib/          # Utilities
│   │   │   ├── pages/        # Page components
│   │   │   ├── env.ts        # Environment
│   │   │   └── ...
│   │   ├── public/            # Static files
│   │   ├── vite.config.ts    # Vite configuration
│   │   └── package.json
│   │
│   ├── server/                # Backend API (Hono + Node.js)
│   │   ├── src/
│   │   │   ├── actions/      # API actions
│   │   │   ├── lib/          # Utilities
│   │   │   ├── middlewares/  # Middleware
│   │   │   ├── modules/      # API modules (25+)
│   │   │   ├── routes.ts     # Route definitions
│   │   │   └── ...
│   │   └── package.json
│   │
│   ├── admin/                 # Admin dashboard
│   ├── custom-domain-edge/    # Custom domain edge
│   ├── docs/                  # Documentation site
│   └── worker/                # Background worker
│
├── packages/
│   ├── auth/                  # Authentication
│   ├── bullmq/               # Job queue (BullMQ)
│   ├── clickhouse/           # Analytics
│   ├── database/             # Database (Prisma)
│   ├── design-system/        # Design system components
│   ├── i18n/                 # Internationalization
│   ├── logger/               # Logging
│   ├── shared/               # Shared utilities
│   ├── storage/              # Storage
│   ├── usage/                # Usage tracking
│   └── validators/           # Validation
│
└── docs/                     # Documentation
```

---

## 🔍 Frontend Analysis (apps/app/)

### Current Stack

| Technology                 | Version  | Purpose                    |
| -------------------------- | -------- | -------------------------- |
| **Vite**                   | Latest   | Build tool                 |
| **@tanstack/react-start**  | 1.168.48 | Full-stack React framework |
| **React**                  | 18.x     | UI library                 |
| **TypeScript**             | Latest   | Type system                |
| **Tailwind CSS**           | Latest   | Styling                    |
| **Nitro**                  | Latest   | Server framework (SSR)     |
| **MDX**                    | 3.x      | Markdown + JSX             |
| **Paraglide**              | Latest   | i18n                       |
| **@tanstack/react-query**  | 5.x      | Data fetching              |
| **@tanstack/react-router** | 1.x      | Routing                    |

### Key Features

1. **Full-Stack Framework**: Uses @tanstack/react-start for SSR
2. **MDX Support**: Documentation written in MDX (Markdown + JSX)
3. **Internationalization**: Paraglide for i18n with multiple languages
4. **Server Functions**: SSR with server-side data fetching
5. **Proxy Configuration**: Proxies `/api/**` to backend server
6. **Security Headers**: Built-in CSP, HSTS, etc.
7. **Rate Limiting**: Built-in rate limiting

### Build Configuration

**vite.config.ts** highlights:

- Uses `@tanstack/react-start/plugin/vite` for SSR
- Uses `nitro/vite` for server framework
- Proxies `/api/**` to `VITE_API_URL` (default: http://localhost:4311)
- Security headers configuration
- MDX plugin for documentation
- Tailwind CSS plugin
- Paraglide i18n plugin

### Source Structure

```
apps/app/src/
├── components/           # React components (50+ files)
│   ├── app-providers.tsx
│   ├── auth-providers.tsx
│   ├── markdown.tsx
│   ├── error-page.tsx
│   └── ...
│
├── functions/            # Server functions (SSR)
│   ├── marketing.ts
│   ├── session.ts
│   └── site.ts
│
├── layouts/              # Layout components
│   ├── auth.tsx
│   ├── dashboard.tsx
│   └── project.tsx
│
├── lib/                  # Utilities (50+ files)
│   ├── auth-errors.ts
│   ├── blog.ts
│   ├── content-security-policy.ts
│   ├── format.ts
│   └── ...
│
├── pages/                # Page components
│   ├── admin/
│   ├── app/
│   ├── auth/
│   ├── marketing/
│   └── ...
│
├── env.ts                # Environment variables
├── env.server.ts          # Server environment
└── ...
```

### Dependencies Summary

**Core Dependencies:**

- `@tanstack/react-start`: Full-stack framework
- `@tanstack/react-query`: Data fetching
- `@tanstack/react-router`: Routing
- `@tanstack/react-form`: Form handling
- `@t3-oss/env-core`: Environment variables
- `@scalar/api-reference-react`: API documentation UI
- `@mdx-js/rollup`: MDX support
- `nitro`: Server framework
- `tailwindcss`: Styling
- `paraglide`: i18n

**UI Libraries:**

- `@dnd-kit/core`, `@dnd-kit/sortable`, `@dnd-kit/utilities`: Drag and drop
- `@tiptap/core`, `@tiptap/extension-*`: Rich text editor
- `@radix-ui/*`: Accessible UI components
- `lucide-react`: Icons

---

## 🔍 Backend Analysis (apps/server/)

### Current Stack

| Technology            | Version | Purpose                |
| --------------------- | ------- | ---------------------- |
| **Hono**              | 2.x     | Web framework          |
| **@hono/node-server** | 2.x     | Node.js server adapter |
| **Prisma**            | Latest  | ORM                    |
| **Better Auth**       | Latest  | Authentication         |
| **BullMQ**            | Latest  | Job queue (Redis)      |
| **ClickHouse**        | Latest  | Analytics              |
| **PostgreSQL**        | 14+     | Primary database       |

### API Structure

```
apps/server/src/
├── actions/              # API actions
├── database/             # Database setup
├── errors/               # Error handling
├── lib/                  # Utilities
├── middlewares/          # Middleware
│   └── rate-limit.ts     # Rate limiting
├── modules/              # API modules (25+)
│   ├── admin/
│   ├── app/
│   │   ├── addons/
│   │   ├── ai/
│   │   ├── analytics/
│   │   ├── api-keys/
│   │   ├── assets/
│   │   ├── branches/
│   │   ├── comments/
│   │   ├── deployments/
│   │   ├── domains/
│   │   ├── exports/
│   │   ├── git/
│   │   ├── imports/
│   │   ├── integrations/
│   │   ├── languages/
│   │   ├── members/
│   │   ├── notifications/
│   │   ├── openapi/
│   │   ├── pages/
│   │   ├── project-members/
│   │   ├── project-settings/
│   │   └── projects/
│   ├── mcp/
│   └── public/
├── routes.ts             # Route definitions
├── server.ts             # Server setup
└── ...
```

### API Routes

**Base URL**: `/api`

**Modules:**

- `/auth/*` - Authentication (Better Auth)
- `/mcp` - MCP protocol
- `/app/*` - Main application API (25+ submodules)
- `/admin/*` - Admin API
- `/public/*` - Public API

**Key Endpoints:**

- `GET /health` - Health check
- `GET /docs` - API documentation (Scalar)
- `GET /openapi.json` - OpenAPI spec

---

## 🔍 Database Analysis (packages/database/)

### Schema

**Prisma Schema**: `packages/database/prisma/schema.prisma`

**Tables** (40+ tables):

- User
- Organization
- Project
- Page
- Branch
- Language
- GitConnection
- Integration
- Deployment
- Domain
- ReaderAccess
- Comment
- SearchIndex
- Asset
- Usage
- Notification
- PlatformEvent
- MCP-related tables
- And more...

**Database**: PostgreSQL with Prisma ORM

---

## 🎯 Migration Strategy

### Current State

| Aspect         | Original                     | Our Rust Implementation |
| -------------- | ---------------------------- | ----------------------- |
| **Backend**    | Node.js + Hono               | Rust + Axum             |
| **Frontend**   | Vite + @tanstack/react-start | Need to migrate         |
| **Database**   | PostgreSQL + Prisma          | PostgreSQL + SQLx       |
| **Auth**       | Better Auth                  | Custom (in-process)     |
| **API Routes** | `/api/*`                     | `/api/*` (matching)     |
| **Port**       | 4311                         | 3000 (configurable)     |

### Key Insight

**The original frontend already uses Vite!** We don't need to migrate the build system - we need to:

1. **Copy** the frontend from `cms-original/apps/app/`
2. **Update** API endpoints to match Rust backend
3. **Update** authentication to work with Rust JWT/sessions
4. **Update** environment configuration
5. **Proxy** API requests to Rust server during development
6. **Build** frontend to be served by Rust in production

---

## 📋 Migration Plan

### Phase 1: Setup (1-2 hours)

1. **Copy frontend to our workspace**

   ```bash
   cp -r /home/user/cms-original/apps/app /home/user/cms-rs/frontend-app
   ```

2. **Update package.json**
   - Change name from `@cms/app` to `cms-app`
   - Update dependencies (remove workspace:\* references)
   - Add our Rust backend as API dependency

3. **Update Vite configuration**
   - Change proxy target from `http://localhost:4311` to `http://localhost:3000`
   - Update environment variables
   - Remove Nitro server (use Rust backend instead)

### Phase 2: API Endpoint Mapping (2-4 hours)

**Original API** (Hono):

- `/api/auth/*` → Better Auth
- `/api/app/*` → Main app API
- `/api/admin/*` → Admin API
- `/api/public/*` → Public API

**Our Rust API** (Axum):

- `/api/auth/*` → Our auth handlers ✅
- `/api/orgs/*` → Organizations (was `/api/app/orgs/*`)
- `/api/projects/*` → Projects (was `/api/app/projects/*`)
- `/api/pages/*` → Pages (was `/api/app/pages/*`)
- `/api/admin/*` → Admin ✅
- `/api/public/*` → Public ✅

**Mapping Strategy:**

| Original Endpoint         | Rust Endpoint         | Action          |
| ------------------------- | --------------------- | --------------- |
| `/api/auth/*`             | `/api/auth/*`         | ✅ Direct match |
| `/api/app/orgs/*`         | `/api/orgs/*`         | Update frontend |
| `/api/app/projects/*`     | `/api/projects/*`     | Update frontend |
| `/api/app/pages/*`        | `/api/pages/*`        | Update frontend |
| `/api/app/branches/*`     | `/api/branches/*`     | Update frontend |
| `/api/app/languages/*`    | `/api/languages/*`    | Update frontend |
| `/api/app/git/*`          | `/api/git/*`          | Update frontend |
| `/api/app/integrations/*` | `/api/integrations/*` | Update frontend |
| `/api/app/deployments/*`  | `/api/deployments/*`  | Update frontend |
| `/api/app/domains/*`      | `/api/domains/*`      | Update frontend |
| `/api/app/comments/*`     | `/api/comments/*`     | Update frontend |
| `/api/app/assets/*`       | `/api/assets/*`       | Update frontend |
| `/api/admin/*`            | `/api/admin/*`        | ✅ Direct match |
| `/api/public/*`           | `/api/public/*`       | ✅ Direct match |

**Implementation:**

- Create a mapping utility in frontend
- Update all API calls to use new endpoints
- Search and replace `/api/app/` with `/api/`

### Phase 3: Authentication Migration (2-3 hours)

**Original Auth**: Better Auth (OAuth, email/password, sessions)

**Our Auth**: Custom in-process (JWT, sessions, API keys, basic)

**Migration Steps:**

1. **Update useAuth hook**
   - Change from Better Auth API to our Rust auth endpoints
   - Update login/register/logout endpoints
   - Update token storage (JWT vs. session cookies)

2. **Update API client**
   - Add auth token to requests (Bearer token or session cookie)
   - Handle 401 responses (clear token, redirect to login)

3. **Update auth providers**
   - Remove Better Auth dependencies
   - Use our Rust auth endpoints

**Endpoint Mapping:**

| Action      | Original                       | Rust                      | Status    |
| ----------- | ------------------------------ | ------------------------- | --------- |
| Login       | `POST /api/auth/sign-in/email` | `POST /api/auth/login`    | ⚠️ Update |
| Register    | `POST /api/auth/sign-up/email` | `POST /api/auth/register` | ⚠️ Update |
| Logout      | `POST /api/auth/sign-out`      | `POST /api/auth/logout`   | ⚠️ Update |
| Get Session | `GET /api/auth/session`        | `GET /api/auth/me`        | ⚠️ Update |
| Refresh     | `POST /api/auth/refresh`       | `POST /api/auth/refresh`  | ✅ Match  |

### Phase 4: Environment Configuration (1 hour)

**Update environment variables:**

| Variable       | Original              | Rust                  | Action |
| -------------- | --------------------- | --------------------- | ------ |
| `VITE_API_URL` | http://localhost:4311 | http://localhost:3000 | Update |
| `VITE_APP_URL` | https://cms.com       | Configurable          | Keep   |
| Auth-related   | Better Auth           | Our auth              | Update |

### Phase 5: Build and Integration (1-2 hours)

1. **Install frontend dependencies**

   ```bash
   cd frontend-app
   npm install
   ```

2. **Build frontend**

   ```bash
   npm run build
   ```

3. **Serve frontend from Rust**
   - Configure Rust to serve static files from `frontend-app/dist`
   - Set up fallback to `index.html` for SPA routing
   - Configure CORS for development

4. **Proxy configuration**
   - Vite proxy to Rust during development
   - Rust serves frontend in production

### Phase 6: Testing and Validation (2-4 hours)

1. **Test all pages**
   - Login, register, dashboard, projects, pages, etc.
   - Verify all API calls work with new endpoints

2. **Test authentication**
   - Login with email/password
   - Session persistence
   - Token refresh
   - Logout

3. **Test edge cases**
   - 401 handling
   - Rate limiting
   - Error responses

---

## 📊 Estimated Timeline

| Phase     | Tasks                     | Estimated Time | Priority |
| --------- | ------------------------- | -------------- | -------- |
| 1         | Setup                     | 1-2 hours      | High     |
| 2         | API Endpoint Mapping      | 2-4 hours      | High     |
| 3         | Authentication Migration  | 2-3 hours      | High     |
| 4         | Environment Configuration | 1 hour         | Medium   |
| 5         | Build and Integration     | 1-2 hours      | High     |
| 6         | Testing and Validation    | 2-4 hours      | High     |
| **Total** |                           | **9-16 hours** |          |

---

## 🎯 Recommended Approach

### Option A: Direct Migration (Recommended)

**Steps:**

1. Copy `cms-original/apps/app/` to `cms-rs/frontend-app/`
2. Update all API endpoints from `/api/app/*` to `/api/*`
3. Update authentication to use Rust endpoints
4. Update environment configuration
5. Configure Vite proxy to Rust server
6. Configure Rust to serve frontend in production
7. Test thoroughly

**Pros:**

- ✅ Preserves all existing UI/UX
- ✅ Minimal changes to frontend code
- ✅ Maintains user familiarity
- ✅ Faster migration

**Cons:**

- ⚠️ Need to remove Better Auth dependencies
- ⚠️ Need to update server functions (SSR won't work with Rust)

### Option B: Incremental Migration

**Steps:**

1. Copy frontend to workspace
2. Migrate one page/module at a time
3. Update API endpoints incrementally
4. Test each page as we go
5. Wire to Rust backend gradually

**Pros:**

- ✅ Lower risk (incremental changes)
- ✅ Can validate each step
- ✅ Easier to debug

**Cons:**

- ❌ Takes longer
- ❌ Need to maintain both old and new endpoints temporarily

### Option C: Hybrid (Recommended for Production)

**Steps:**

1. Copy frontend to workspace
2. **Remove SSR** (server functions, Nitro) - use client-side only
3. Update all API endpoints
4. Update authentication
5. Configure Vite for client-side only
6. Serve from Rust in production

**Pros:**

- ✅ Simplest approach
- ✅ No need to maintain SSR with Rust
- ✅ Clean separation
- ✅ Easier to maintain

**Cons:**

- ⚠️ Loses SSR benefits (SEO, performance)
- ⚠️ Need to handle more on client side

---

## 💡 My Recommendation

**Option C: Hybrid (Client-side only)**

**Rationale:**

1. **Simpler**: No need to integrate SSR with Rust
2. **Faster**: Can be done in 1-2 days
3. **Cleaner**: Clear separation between frontend and backend
4. **Production-ready**: Works well for documentation sites
5. **SEO**: Documentation sites can use pre-rendering or static generation

**Implementation Steps:**

1. **Copy frontend**

   ```bash
   cp -r /home/user/cms-original/apps/app /home/user/cms-rs/frontend-app
   ```

2. **Simplify frontend**
   - Remove `@tanstack/react-start` (SSR framework)
   - Remove `nitro` (server framework)
   - Remove server functions
   - Keep client-side React only

3. **Update API endpoints**
   - Search and replace `/api/app/` with `/api/`
   - Update auth endpoints
   - Update environment variables

4. **Update authentication**
   - Replace Better Auth with our Rust auth
   - Update useAuth hook
   - Update API client

5. **Configure Vite**
   - Simple Vite config with proxy to Rust
   - No SSR plugins
   - Standard React setup

6. **Integrate with Rust**
   - Serve frontend from Rust in production
   - Configure CORS
   - Set up fallback to index.html

---

## 📝 Next Steps

**Would you like me to:**

1. **Proceed with Option C** - Copy and migrate the frontend now
2. **Create detailed migration scripts** - Step-by-step scripts for each phase
3. **Analyze specific files** - Deep dive into particular frontend components
4. **Create a comparison document** - Side-by-side comparison of original vs. our implementation

**My recommendation**: Proceed with **Option C** (Hybrid approach) and start the migration now. I can copy the frontend, simplify it, update the API endpoints, and integrate it with our Rust backend.

Would you like me to proceed with the migration?
