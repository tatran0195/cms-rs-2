import { describe, expect, it } from 'vitest';
import { publishedSiteLogo } from './site-branding';

describe('publishedSiteLogo', () => {
  it('uses the mark-only asset for the official cms documentation wordmark', () => {
    expect(publishedSiteLogo('https://cms.com/brand/raster/logo/cms-logo-horizontal-ltr.png', 'light')).toEqual({
      src: 'https://cms.com/brand/cms-icon.svg',
      markOnly: true,
    });
    expect(publishedSiteLogo('https://cms.com/brand/raster/logo/cms-logo-horizontal-ltr-reverse.png', 'dark')).toEqual({
      src: 'https://cms.com/brand/cms-icon-reverse.svg',
      markOnly: true,
    });
  });

  it('keeps customer logos and their adjacent site name unchanged', () => {
    expect(publishedSiteLogo('https://assets.example.com/logo.svg', 'light')).toEqual({
      src: 'https://assets.example.com/logo.svg',
      markOnly: false,
    });
  });

  it('keeps the empty branding fallback available', () => {
    expect(publishedSiteLogo(null, 'light')).toBeNull();
  });
});
