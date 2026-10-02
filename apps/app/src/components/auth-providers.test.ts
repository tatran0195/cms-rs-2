import { describe, expect, it } from 'vitest';
import { authDocumentTitle } from '@/lib/auth-document-title';

const translate = (key: string) =>
  ({
    'auth.signIn.submit': 'لاگ ان کریں۔',
    'auth.signUp.submit': 'اکاؤنٹ بنائیں',
    'auth.verify.title': 'اپنے ای میل کی تصدیق کریں',
    'auth.passwordless.subtitle': 'cms پاس ورڈ کے بغیر ہے۔',
  })[key] ?? key;

describe('authDocumentTitle', () => {
  it.each([
    ['/sign-in', 'لاگ ان کریں۔ — cms'],
    ['/sign-up', 'اکاؤنٹ بنائیں — cms'],
    ['/verify-email', 'اپنے ای میل کی تصدیق کریں — cms'],
    ['/forgot-password', 'cms پاس ورڈ کے بغیر ہے۔ — cms'],
    ['/reset-password', 'cms پاس ورڈ کے بغیر ہے۔ — cms'],
  ])('localizes %s', (pathname, expected) => {
    expect(authDocumentTitle(pathname, translate)).toBe(expected);
  });

  it('leaves unrelated routes to their own head metadata', () => {
    expect(authDocumentTitle('/pricing', translate)).toBeNull();
  });
});
