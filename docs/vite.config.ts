import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { rari, rariRouter } from "rari";
import { defineConfig } from "rolldown-vite";

export default defineConfig({
  plugins: [rari(), rariRouter(), react(), tailwindcss()],
});
