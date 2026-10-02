import { useQuery } from '@tanstack/react-query';
import { queryClient } from '@/lib/query-client';

export interface User {
  id: string;
  email: string;
  name?: string;
  image?: string;
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

async function authFetch<T>(path: string, options?: RequestInit): Promise<{ data?: T; error?: { message?: string; code?: string } }> {
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
      return {
        error: {
          message: err?.message || err?.error?.message || `HTTP ${res.status}`,
          code: err?.code || err?.error?.code,
        },
      };
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
  useSession,
  verifyEmail: async (args: { query: { token: string } }) =>
    authFetch<{ success?: boolean }>(`/api/auth/verify-email?token=${encodeURIComponent(args.query.token)}`, {
      method: 'GET',
    }),
  emailOtp: {
    sendVerificationOtp: async (args: { email: string; type?: string }) =>
      authFetch<{ success?: boolean }>('/api/auth/email-otp/send-verification-otp', {
        method: 'POST',
        body: JSON.stringify(args),
      }),
    verifyEmail: async (args: { email: string; otp: string }) =>
      authFetch<{ success?: boolean }>('/api/auth/email-otp/verify-email', {
        method: 'POST',
        body: JSON.stringify(args),
      }),
    requestEmailChange: async (args: { newEmail: string; otp: string }) =>
      authFetch<{ success?: boolean }>('/api/auth/email-otp/request-email-change', {
        method: 'POST',
        body: JSON.stringify(args),
      }),
    changeEmail: async (args: { newEmail: string; otp: string }) =>
      authFetch<{ success?: boolean }>('/api/auth/email-otp/change-email', {
        method: 'POST',
        body: JSON.stringify(args),
      }),
  },
  signIn: {
    emailOtp: async (args: { email: string; otp: string; name?: string }) => {
      const result = await authFetch<SessionData>('/api/auth/sign-in/email-otp', {
        method: 'POST',
        body: JSON.stringify(args),
      });
      if (result.data) {
        queryClient.invalidateQueries({ queryKey: sessionQueryKey });
      }
      return result;
    },
    social: async (args: { provider: string; callbackURL?: string }) => {
      const result = await authFetch<{ url?: string }>('/api/auth/sign-in/social', {
        method: 'POST',
        body: JSON.stringify(args),
      });
      if (result.data?.url && typeof window !== 'undefined') {
        window.location.href = result.data.url;
      }
      return result;
    },
  },
  signOut: async () => {
    const result = await authFetch<{ success?: boolean }>('/api/auth/sign-out', { method: 'POST' });
    queryClient.setQueryData(sessionQueryKey, null);
    queryClient.clear();
    return result;
  },
  updateUser: async (args: { name?: string; image?: string }) =>
    authFetch<{ success?: boolean }>('/api/auth/update-user', { method: 'POST', body: JSON.stringify(args) }),
  organization: {
    acceptInvitation: async (args: { invitationId: string }) =>
      authFetch<{ success?: boolean }>('/api/auth/organizations/accept-invitation', { method: 'POST', body: JSON.stringify(args) }),
  },
  admin: {
    stopImpersonating: async () =>
      authFetch<{ success?: boolean }>('/api/auth/admin/stop-impersonating', { method: 'POST' }),
  },
};

export const signIn = {
  social: authClient.signIn.social,
  emailOtp: authClient.signIn.emailOtp,
};
