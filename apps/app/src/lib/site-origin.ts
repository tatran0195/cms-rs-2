/**
 * In the client SPA, custom domain origin is simply window.location.origin.
 */
export const customDomainOrigin = (): string | undefined => {
  return typeof window !== 'undefined' ? window.location.origin : undefined;
};
