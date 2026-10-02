import { ThemeProvider } from "@cms/design-system/theme";
import {
  createRootRoute,
  HeadContent,
  Outlet,
  useRouterState,
} from "@tanstack/react-router";
import { useEffect, type ReactNode } from "react";
import { RootMarketingAnalytics } from "@/components/root-marketing-analytics";
import type { SiteShell } from "@/hooks/api/types";
import { siteThemeNoFlashScript } from "@/lib/site-theme";
import appCss from "@/styles.css?url";

export const Route = createRootRoute({
  head: () => ({
    meta: [
      { charSet: "utf-8" },
      { name: "viewport", content: "width=device-width, initial-scale=1" },
      { title: "cms Cloud — documentation workspace" },
      { name: "application-name", content: "cms" },
      { name: "theme-color", content: "#181612" },
    ],
    links: [
      { rel: "stylesheet", href: appCss },
      { rel: "icon", href: "/favicon.svg", type: "image/svg+xml" },
      {
        rel: "icon",
        href: "/favicon-32x32.png",
        type: "image/png",
        sizes: "32x32",
      },
      { rel: "apple-touch-icon", href: "/apple-touch-icon.png" },
      { rel: "manifest", href: "/site.webmanifest" },
    ],
  }),
  component: RootComponent,
});

function RootComponent() {
  return (
    <RootDocument>
      <Outlet />
    </RootDocument>
  );
}

function RootDocument({ children }: { children: ReactNode }) {
  const { lang, dir, pathname, siteProjectId, siteThemeDefault } =
    useRouterState({
      select: (state) => {
        const match = state.matches.find(
          (m) => m.routeId === "/sites/$projectId",
        );
        const site = (match?.loaderData as { site?: SiteShell } | undefined)
          ?.site;
        if (!site) {
          const articleMatch = state.matches.find(
            (m) => m.routeId === "/blog/$slug",
          );
          const language =
            (articleMatch?.loaderData as { language?: "ar" | "en" } | undefined)
              ?.language ?? "en";
          const arabicMarketingRoute =
            state.location.pathname === "/ar" ||
            state.location.pathname.startsWith("/ar/");
          if (language === "ar" || arabicMarketingRoute) {
            return {
              lang: "ar",
              dir: "rtl" as const,
              pathname: state.location.pathname,
              siteProjectId: undefined,
              siteThemeDefault: undefined,
            };
          }
          return {
            lang: "en",
            dir: "ltr" as const,
            pathname: state.location.pathname,
            siteProjectId: undefined,
            siteThemeDefault: undefined,
          };
        }
        const code =
          (state.location.search as { lang?: string }).lang ??
          site.activeLanguage;
        const active =
          site.languages.find((l) => l.code === code) ??
          site.languages.find((l) => l.isDefault) ??
          site.languages[0];
        return {
          lang: active?.code ?? "en",
          dir:
            active?.direction === "RTL" ? ("rtl" as const) : ("ltr" as const),
          pathname: state.location.pathname,
          siteProjectId: (match?.params as { projectId?: string } | undefined)
            ?.projectId,
          siteThemeDefault: site.project.config?.styling?.theme ?? "light",
        };
      },
    });

  useEffect(() => {
    document.documentElement.lang = lang;
    document.documentElement.dir = dir;
  }, [lang, dir]);

  const siteThemeBootstrap = siteProjectId
    ? siteThemeNoFlashScript(siteProjectId, siteThemeDefault ?? "light")
    : null;

  return (
    <>
      <HeadContent />
      <ThemeProvider
        applyDocumentTheme={!siteProjectId}
        initialThemeScript={siteThemeBootstrap ?? undefined}
      >
        {children}
      </ThemeProvider>
      <RootMarketingAnalytics
        pathname={pathname}
        siteProjectId={siteProjectId}
        language={lang === "ar" ? "ar" : "en"}
      />
    </>
  );
}
