import { siteT } from '@cms/i18n/site';
import type { BlogEntry } from './blog';
import { breadcrumbLd, canonicalHref, faqLd, pageMeta } from './marketing-seo';

/** SEO metadata and structured data for one blog article. Kept independent of
 * the article UI so route head evaluation does not pull the renderer bundle. */
export function articleHead(entry: BlogEntry, translation?: BlogEntry) {
  const path = `/blog/${entry.slug}`;
  const language = entry.language ?? 'en';
  const arabic = language === 'ar';
  const t = siteT(language);
  const imagePath = arabic ? '/brand/raster/social/cms-og-card-ar.png' : '/brand/raster/social/cms-og-card.png';
  const scripts = [
    {
      type: 'application/ld+json',
      children: JSON.stringify({
        '@context': 'https://schema.org',
        '@type': 'Article',
        headline: entry.title,
        description: entry.description,
        datePublished: entry.datePublished,
        dateModified: entry.dateModified,
        inLanguage: language,
        mainEntityOfPage: canonicalHref(path),
        image: canonicalHref(imagePath),
        author: { '@type': 'Organization', name: 'cms', url: canonicalHref('/about') },
        publisher: {
          '@type': 'Organization',
          name: 'cms',
          logo: { '@type': 'ImageObject', url: canonicalHref('/brand/raster/logo/cms-logo-horizontal-ltr.png') },
        },
      }),
    },
    breadcrumbLd([
      { name: t('home'), path: arabic ? '/ar' : '/' },
      { name: t('blog'), path: '/blog' },
      { name: entry.title, path },
    ]),
    ...(entry.faqs && entry.faqs.length > 0 ? [faqLd(entry.faqs.map((faq) => ({ q: faq.question, a: faq.answer })))] : []),
  ];

  return {
    meta: pageMeta({
      title: entry.metaTitle ?? `${entry.title} | cms`,
      description: entry.description,
      path,
      type: 'article',
      locale: arabic ? 'ar_AR' : 'en_US',
      imagePath,
      imageAlt: arabic ? `cms: ${entry.title}` : undefined,
    }),
    links: [
      { rel: 'canonical', href: canonicalHref(path) },
      ...(translation
        ? [
          { rel: 'alternate', hrefLang: language, href: canonicalHref(path) },
          { rel: 'alternate', hrefLang: translation.language ?? 'en', href: canonicalHref(`/blog/${translation.slug}`) },
          {
            rel: 'alternate',
            hrefLang: 'x-default',
            href: canonicalHref(`/blog/${language === 'en' ? entry.slug : translation.slug}`),
          },
        ]
        : []),
    ],
    scripts,
  };
}
