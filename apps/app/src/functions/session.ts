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
