import { getData } from '@/hooks/api/client-helpers';
import type { ChangelogEntry, SitePage, SiteShell } from '@/hooks/api/types';
import { api } from '@/services/api';

export const getSiteFn = async ({
  data,
}: {
  data: { projectId: string; language?: string; version?: string };
}): Promise<SiteShell> =>
  getData<SiteShell>(
    await (api as any).public.sites[':id'].$get({
      param: { id: data.projectId },
      query: { ...(data.language ? { lang: data.language } : {}), ...(data.version ? { version: data.version } : {}) },
    }),
    'site',
  );

export const getSitePageFn = async ({
  data,
}: {
  data: { projectId: string; path: string; language?: string; version?: string };
}): Promise<SitePage> =>
  getData<SitePage>(
    await (api as any).public.sites[':id'].page.$get({
      param: { id: data.projectId },
      query: {
        path: data.path,
        ...(data.language ? { lang: data.language } : {}),
        ...(data.version ? { version: data.version } : {}),
      },
    }),
    'page',
  );

export const listSiteChangelogFn = async ({
  data,
}: {
  data: { projectId: string };
}): Promise<ChangelogEntry[]> =>
  getData<ChangelogEntry[]>(
    await (api as any).public.sites[':id'].changelog.$get({ param: { id: data.projectId } }),
    'changelog',
  );

export const getGitPreviewFn = async ({
  data,
}: {
  data: { token: string };
}): Promise<string> => {
  const response = await (api as any).public.git.previews[':token'].$get({ param: { token: data.token } });
  return JSON.stringify(await getData(response, 'pull-request preview'));
};
