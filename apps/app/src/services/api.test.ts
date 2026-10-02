import { describe, it, expect, vi, beforeEach } from 'vitest';
import { api } from './api';

describe('API Proxy Client', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('builds correct URL and sends GET request with params and query', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({ data: { id: 'proj-123' } })));
    vi.stubGlobal('fetch', fetchMock);

    const res = await (api as any).app.projects[':id'].$get({
      param: { id: 'proj-123' },
      query: { lang: 'en', limit: 10 },
    });

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0] as [any, any];
    expect(url.toString()).toContain('/api/app/projects/proj-123?lang=en&limit=10');
    expect(init.method).toBe('GET');
    expect(init.credentials).toBe('include');
    expect(res).toBeInstanceOf(Response);
  });

  it('builds correct POST request with json body and content-type', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({ data: { success: true } })));
    vi.stubGlobal('fetch', fetchMock);

    await (api as any).app.projects[':projectId'].pages.$post({
      param: { projectId: 'p1' },
      json: { title: 'New Page' },
    });

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0] as [any, any];
    expect(url.toString()).toContain('/api/app/projects/p1/pages');
    expect(init.method).toBe('POST');
    expect(init.headers.get('Content-Type')).toBe('application/json');
    expect(JSON.parse(init.body)).toEqual({ title: 'New Page' });
  });

  it('handles DELETE request', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({ data: { deleted: true } })));
    vi.stubGlobal('fetch', fetchMock);

    await (api as any).app.projects[':projectId'].branches[':id'].$delete({
      param: { projectId: 'p1', id: 'b1' },
    });

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0] as [any, any];
    expect(url.toString()).toContain('/api/app/projects/p1/branches/b1');
    expect(init.method).toBe('DELETE');
  });
});
