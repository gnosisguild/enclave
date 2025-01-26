import { readdirSync } from "fs";
import { dirname, resolve } from "path";
import { fileURLToPath } from "url";
import { defineConfig } from "vitepress";
import type { DefaultTheme } from "vitepress";

const __dirname = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  title: "Enclave docs",
  themeConfig: {
    sidebar: generateSidebar(),
  } satisfies DefaultTheme.Config,
});

function generateSidebar(): DefaultTheme.SidebarItem[] {
  const docsDir = resolve(__dirname, "../");
  return readdirSync(docsDir)
    .filter((file) => file.endsWith(".md"))
    .map((file) => ({
      text: file.replace(".md", ""),
      link: "/" + file.replace(".md", ""),
    }));
}
