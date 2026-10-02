import { getLocale, REQUEST_LOCALE_HEADER } from '@cms/i18n';

interface RequestArgs {
  param?: Record<string, string | number>;
  query?: Record<string, string | number | boolean | undefined | null>;
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
            // Locale might not be loaded yet
          }

          if (args?.json !== undefined) {
            if (!headers.has('Content-Type')) {
              headers.set('Content-Type', 'application/json');
            }
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
