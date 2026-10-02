// @vitest-environment jsdom

import { renderToStaticMarkup } from "react-dom/server";
import { expect, it } from "vitest";
import { DeveloperResourcesPage } from "./developers";

it("publishes named cms developer and agent entry points with an explicit auth boundary", () => {
  const html = renderToStaticMarkup(<DeveloperResourcesPage />);
  expect(html).toContain("cms developer resources");
  expect(html).toContain('href="/openapi.json"');
  expect(html).toContain('href="/llms.txt"');
  expect(html).toContain("@cms/cli");
  expect(html).toContain("not a supported third-party write API");
});
