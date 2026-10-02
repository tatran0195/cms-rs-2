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

  it('signIn.emailOtp calls /api/auth/sign-in/email-otp', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({
      user: { id: 'u1', email: 'test@example.com' },
      session: { id: 's1', userId: 'u1' },
    })));
    vi.stubGlobal('fetch', fetchMock);

    const res = await signIn.emailOtp({
      email: 'test@example.com',
      otp: '123456',
    });

    expect(fetchMock).toHaveBeenCalledWith('/api/auth/sign-in/email-otp', expect.objectContaining({
      method: 'POST',
      body: JSON.stringify({ email: 'test@example.com', otp: '123456' }),
      credentials: 'include',
    }));
    expect(res.data?.user.id).toBe('u1');
  });
});
