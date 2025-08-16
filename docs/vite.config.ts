import tailwindcss from "@tailwindcss/vite";

import { rari, rariRouter } from "rari";
import { defineConfig } from "rolldown-vite";

export default defineConfig({
  plugins: [rari(), rariRouter(), tailwindcss()],
});
