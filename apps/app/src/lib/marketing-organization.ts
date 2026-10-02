/** Public organization facts shared by every cms JSON-LD graph.
 * Keep these values limited to facts cms intentionally publishes. */
export const CMS_ORGANIZATION = {
  name: 'cms',
  address: {
    '@type': 'PostalAddress',
    addressCountry: 'TN',
  },
  supportContact: {
    '@type': 'ContactPoint',
    contactType: 'Product support',
    email: 'support@cms.com',
    availableLanguage: ['English', 'Arabic'],
  },
} as const;
