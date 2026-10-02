/** @vitest-environment jsdom */

import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PageNode } from "@/hooks/api";
import {
  flatten,
  getProjection,
  hideCollapsed,
  removeDescendants,
  reorderTree,
  SortablePageTree,
} from "./sortable-page-tree";

vi.mock("@cms/i18n/react", () => ({ useT: () => (key: string) => key }));
vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);

const roots: Array<ReturnType<typeof createRoot>> = [];

beforeEach(() => {
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe = vi.fn();
      unobserve = vi.fn();
      disconnect = vi.fn();
    },
  );
  window.localStorage.clear();
});

afterEach(async () => {
  for (const root of roots.splice(0)) {
    await act(async () => root.unmount());
  }
  document.body.replaceChildren();
  vi.clearAllMocks();
});

function makeNode(partial: Partial<PageNode> & { id: string }): PageNode {
  return {
    parentId: null,
    languageId: "lang-en",
    kind: "PAGE",
    title: partial.id,
    slug: partial.id,
    path: `/${partial.id}`,
    icon: null,
    description: null,
    position: 0,
    hidden: false,
    updatedAt: "2026-01-01T00:00:00Z",
    ...partial,
  };
}

describe("SortablePageTree - Tree Algorithms", () => {
  describe("flatten", () => {
    it("returns empty array for empty pages", () => {
      expect(flatten([])).toEqual([]);
    });

    it("sorts root items by position", () => {
      const pages = [
        makeNode({ id: "p3", position: 2 }),
        makeNode({ id: "p1", position: 0 }),
        makeNode({ id: "p2", position: 1 }),
      ];
      const flat = flatten(pages);
      expect(flat.map((f) => f.id)).toEqual(["p1", "p2", "p3"]);
      expect(flat.map((f) => f.depth)).toEqual([0, 0, 0]);
      expect(flat.map((f) => f.parentId)).toEqual([null, null, null]);
    });

    it("flattens hierarchical tree into preorder depth-first list", () => {
      const pages = [
        makeNode({ id: "g1", kind: "GROUP", position: 0 }),
        makeNode({ id: "g1-c2", parentId: "g1", position: 1 }),
        makeNode({ id: "g1-c1", parentId: "g1", position: 0 }),
        makeNode({ id: "p2", position: 1 }),
      ];
      const flat = flatten(pages);
      expect(flat.map((f) => f.id)).toEqual(["g1", "g1-c1", "g1-c2", "p2"]);
      expect(flat.map((f) => f.depth)).toEqual([0, 1, 1, 0]);
      expect(flat.map((f) => f.parentId)).toEqual(["__root", "g1", "g1", "__root"].map((p) => (p === "__root" ? null : p)));
    });

    it("handles multi-level nesting (root -> group -> subgroup -> page)", () => {
      const pages = [
        makeNode({ id: "root-group", kind: "GROUP", position: 0 }),
        makeNode({ id: "sub-group", parentId: "root-group", kind: "GROUP", position: 0 }),
        makeNode({ id: "leaf-page", parentId: "sub-group", position: 0 }),
        makeNode({ id: "root-page", position: 1 }),
      ];
      const flat = flatten(pages);
      expect(flat.map((f) => f.id)).toEqual(["root-group", "sub-group", "leaf-page", "root-page"]);
      expect(flat.map((f) => f.depth)).toEqual([0, 1, 2, 0]);
    });
  });

  describe("removeDescendants", () => {
    it("excludes direct children and recursive descendants of dragged node", () => {
      const pages = [
        makeNode({ id: "g1", kind: "GROUP", position: 0 }),
        makeNode({ id: "g1-sub", parentId: "g1", kind: "GROUP", position: 0 }),
        makeNode({ id: "g1-sub-leaf", parentId: "g1-sub", position: 0 }),
        makeNode({ id: "p2", position: 1 }),
      ];
      const flat = flatten(pages);
      const pruned = removeDescendants(flat, "g1");
      expect(pruned.map((f) => f.id)).toEqual(["g1", "p2"]);
    });

    it("leaves all items when dragging a leaf node", () => {
      const pages = [
        makeNode({ id: "g1", kind: "GROUP", position: 0 }),
        makeNode({ id: "c1", parentId: "g1", position: 0 }),
        makeNode({ id: "p2", position: 1 }),
      ];
      const flat = flatten(pages);
      const pruned = removeDescendants(flat, "c1");
      expect(pruned.map((f) => f.id)).toEqual(["g1", "c1", "p2"]);
    });
  });

  describe("hideCollapsed", () => {
    it("hides direct children and recursive descendants of collapsed groups", () => {
      const pages = [
        makeNode({ id: "g1", kind: "GROUP", position: 0 }),
        makeNode({ id: "g1-sub", parentId: "g1", kind: "GROUP", position: 0 }),
        makeNode({ id: "g1-leaf", parentId: "g1-sub", position: 0 }),
        makeNode({ id: "p2", position: 1 }),
      ];
      const flat = flatten(pages);
      const visible = hideCollapsed(flat, new Set(["g1"]));
      expect(visible.map((f) => f.id)).toEqual(["g1", "p2"]);
    });

    it("returns original items when collapsed set is empty", () => {
      const pages = [
        makeNode({ id: "g1", kind: "GROUP", position: 0 }),
        makeNode({ id: "c1", parentId: "g1", position: 0 }),
      ];
      const flat = flatten(pages);
      expect(hideCollapsed(flat, new Set())).toEqual(flat);
    });
  });

  describe("getProjection", () => {
    const pages = [
      makeNode({ id: "g1", kind: "GROUP", position: 0 }),
      makeNode({ id: "c1", parentId: "g1", position: 0 }),
      makeNode({ id: "p2", position: 1 }),
    ];
    const flat = flatten(pages);

    it("calculates same-depth position for root swap", () => {
      const proj = getProjection(flat, "p2", "g1", 0, 16);
      expect(proj).toEqual({ depth: 0, parentId: null });
    });

    it("calculates nesting under previous item on positive horizontal drag", () => {
      // Dragging p2 onto c1 with indent 16px (dragOffset = 16)
      const proj = getProjection(flat, "p2", "c1", 16, 16);
      expect(proj.depth).toBe(1);
      expect(proj.parentId).toBe("g1");
    });

    it("clamps max depth to previousItem.depth + 1", () => {
      // Trying to indent 5 levels deep under a root item (depth 0)
      const proj = getProjection(flat, "p2", "g1", 16 * 5, 16);
      // Previous item after move to index 0 is undefined, so max depth is 0
      expect(proj.depth).toBe(0);
      expect(proj.parentId).toBe(null);
    });

    it("handles fallback gracefully if id is not found", () => {
      const proj = getProjection(flat, "unknown", "p2", 0, 16);
      expect(proj).toEqual({ depth: 0, parentId: null });
    });
  });

  describe("reorderTree - Operations and Invariants", () => {
    it("reorders root pages cleanly and updates positions", () => {
      const pages = [
        makeNode({ id: "p1", position: 0 }),
        makeNode({ id: "p2", position: 1 }),
        makeNode({ id: "p3", position: 2 }),
      ];
      const flatFull = flatten(pages);
      const flat = flatFull;

      // Drag p3 to the top (over p1)
      const items = reorderTree({
        flatFull,
        flat,
        activeId: "p3",
        overId: "p1",
        projection: { depth: 0, parentId: null },
      });

      expect(items).toEqual([
        { id: "p3", parentId: null, position: 0 },
        { id: "p1", parentId: null, position: 1 },
        { id: "p2", parentId: null, position: 2 },
      ]);
    });

    it("reorders siblings inside a group without affecting other nodes", () => {
      const pages = [
        makeNode({ id: "g1", kind: "GROUP", position: 0 }),
        makeNode({ id: "c1", parentId: "g1", position: 0 }),
        makeNode({ id: "c2", parentId: "g1", position: 1 }),
        makeNode({ id: "p2", position: 1 }),
      ];
      const flatFull = flatten(pages);
      const flat = flatFull;

      // Drag c2 before c1
      const items = reorderTree({
        flatFull,
        flat,
        activeId: "c2",
        overId: "c1",
        projection: { depth: 1, parentId: "g1" },
      });

      expect(items).toEqual([
        { id: "g1", parentId: null, position: 0 },
        { id: "c2", parentId: "g1", position: 0 },
        { id: "c1", parentId: "g1", position: 1 },
        { id: "p2", parentId: null, position: 1 },
      ]);
    });

    it("moves an entire subtree (group + all children) as a contiguous unit", () => {
      const pages = [
        makeNode({ id: "g1", kind: "GROUP", position: 0 }),
        makeNode({ id: "c1", parentId: "g1", position: 0 }),
        makeNode({ id: "c2", parentId: "g1", position: 1 }),
        makeNode({ id: "p2", position: 1 }),
      ];
      const flatFull = flatten(pages);
      // During drag of g1, descendants are removed from visible flat
      const flat = removeDescendants(flatFull, "g1"); // [g1, p2]

      // Drag g1 down past p2
      const items = reorderTree({
        flatFull,
        flat,
        activeId: "g1",
        overId: "p2",
        projection: { depth: 0, parentId: null },
      });

      expect(items).toEqual([
        { id: "p2", parentId: null, position: 0 },
        { id: "g1", parentId: null, position: 1 },
        { id: "c1", parentId: "g1", position: 0 },
        { id: "c2", parentId: "g1", position: 1 },
      ]);
    });

    it("preserves children of collapsed groups when moving nodes", () => {
      const pages = [
        makeNode({ id: "g1", kind: "GROUP", position: 0 }),
        makeNode({ id: "c1", parentId: "g1", position: 0 }),
        makeNode({ id: "p2", position: 1 }),
      ];
      const flatFull = flatten(pages);
      // g1 is collapsed, so flat only has [g1, p2]
      const flat = hideCollapsed(flatFull, new Set(["g1"]));

      // Drag p2 before g1
      const items = reorderTree({
        flatFull,
        flat,
        activeId: "p2",
        overId: "g1",
        projection: { depth: 0, parentId: null },
      });

      expect(items).toEqual([
        { id: "p2", parentId: null, position: 0 },
        { id: "g1", parentId: null, position: 1 },
        { id: "c1", parentId: "g1", position: 0 },
      ]);
    });

    it("nests a root page into a group", () => {
      const pages = [
        makeNode({ id: "g1", kind: "GROUP", position: 0 }),
        makeNode({ id: "c1", parentId: "g1", position: 0 }),
        makeNode({ id: "p2", position: 1 }),
      ];
      const flatFull = flatten(pages);
      const flat = flatFull;

      // Drag p2 under g1 at depth 1 (over c1, moves before c1)
      const items = reorderTree({
        flatFull,
        flat,
        activeId: "p2",
        overId: "c1",
        projection: { depth: 1, parentId: "g1" },
      });

      expect(items).toEqual([
        { id: "g1", parentId: null, position: 0 },
        { id: "p2", parentId: "g1", position: 0 },
        { id: "c1", parentId: "g1", position: 1 },
      ]);
    });

    it("un-nests a child out of a group to the root level", () => {
      const pages = [
        makeNode({ id: "g1", kind: "GROUP", position: 0 }),
        makeNode({ id: "c1", parentId: "g1", position: 0 }),
        makeNode({ id: "c2", parentId: "g1", position: 1 }),
      ];
      const flatFull = flatten(pages);
      const flat = flatFull;

      // Un-nest c2 to root level (depth 0)
      const items = reorderTree({
        flatFull,
        flat,
        activeId: "c2",
        overId: "c2",
        projection: { depth: 0, parentId: null },
      });

      expect(items).toEqual([
        { id: "g1", parentId: null, position: 0 },
        { id: "c1", parentId: "g1", position: 0 },
        { id: "c2", parentId: null, position: 1 },
      ]);
    });
  });
});

describe("SortablePageTree - React Component Interactions", () => {
  async function renderTree(props: {
    pages: PageNode[];
    activeId?: string | null;
    onSelect?: (id: string) => void;
    onAddChild?: (parentId: string) => void;
    onSettings?: (id: string) => void;
    onMove?: (items: Array<{ id: string; parentId: string | null; position: number }>) => void;
    treeKey?: string;
  }) {
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);
    roots.push(root);

    await act(async () =>
      root.render(
        <SortablePageTree
          pages={props.pages}
          activeId={props.activeId ?? null}
          onSelect={props.onSelect ?? vi.fn()}
          onAddChild={props.onAddChild ?? vi.fn()}
          onSettings={props.onSettings ?? vi.fn()}
          onMove={props.onMove ?? vi.fn()}
          treeKey={props.treeKey}
        />,
      ),
    );
    return container;
  }

  it("renders tree items with indentation and accessible labels", async () => {
    const pages = [
      makeNode({ id: "g1", title: "Getting Started", kind: "GROUP", position: 0 }),
      makeNode({ id: "c1", title: "Installation", parentId: "g1", position: 0 }),
      makeNode({ id: "p2", title: "Architecture", position: 1 }),
    ];

    const container = await renderTree({ pages });

    expect(container.textContent).toContain("Getting Started");
    expect(container.textContent).toContain("Installation");
    expect(container.textContent).toContain("Architecture");

    // Drag handle buttons have touch-none and accessible labels
    const handles = container.querySelectorAll('button[aria-label="editor.dragToReorder"]');
    expect(handles.length).toBe(3);
    for (const handle of handles) {
      expect(handle.className).toContain("touch-none");
    }
  });

  it("triggers onSelect when clicking a page item", async () => {
    const onSelect = vi.fn();
    const pages = [
      makeNode({ id: "p1", title: "Page One", position: 0 }),
      makeNode({ id: "p2", title: "Page Two", position: 1 }),
    ];

    const container = await renderTree({ pages, onSelect });
    const button = Array.from(container.querySelectorAll("button")).find((b) =>
      b.textContent?.includes("Page Two"),
    );

    expect(button).toBeDefined();
    await act(async () => {
      button?.click();
    });

    expect(onSelect).toHaveBeenCalledWith("p2");
  });

  it("triggers onAddChild when clicking the plus button on a group", async () => {
    const onAddChild = vi.fn();
    const pages = [
      makeNode({ id: "g1", title: "API Reference", kind: "GROUP", position: 0 }),
    ];

    const container = await renderTree({ pages, onAddChild });
    const addButton = container.querySelector('button[aria-label="editor.newPage"]');
    expect(addButton).toBeDefined();

    await act(async () => {
      addButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(onAddChild).toHaveBeenCalledWith("g1");
  });

  it("triggers onSettings when clicking the settings button", async () => {
    const onSettings = vi.fn();
    const pages = [
      makeNode({ id: "p1", title: "Guides", position: 0 }),
    ];

    const container = await renderTree({ pages, onSettings });
    const settingsButton = container.querySelector('button[aria-label="editor.pageSettings.title"]');
    expect(settingsButton).toBeDefined();

    await act(async () => {
      settingsButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(onSettings).toHaveBeenCalledWith("p1");
  });

  it("collapses and expands group children and persists to localStorage", async () => {
    const pages = [
      makeNode({ id: "g1", title: "Folder", kind: "GROUP", position: 0 }),
      makeNode({ id: "c1", title: "Child Leaf", parentId: "g1", position: 0 }),
    ];

    const container = await renderTree({ pages, treeKey: "lang-en" });
    expect(container.textContent).toContain("Child Leaf");

    // Click collapse chevron
    const chevron = container.querySelector('button[aria-label="editor.collapse"]');
    expect(chevron).toBeDefined();

    await act(async () => {
      chevron?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    // Child is hidden
    expect(container.textContent).not.toContain("Child Leaf");
    expect(window.localStorage.getItem("cms.editor.collapsedGroups:lang-en")).toContain("g1");

    // Click again to expand
    const expandChevron = container.querySelector('button[aria-label="editor.expand"]');
    await act(async () => {
      expandChevron?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(container.textContent).toContain("Child Leaf");
  });
});
