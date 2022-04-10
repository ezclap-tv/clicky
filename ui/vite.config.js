import { defineConfig } from "vite";
import { replaceCodePlugin } from "vite-plugin-replace";

const API_URL = process.env.API_URL ?? "http://localhost:8080";

export default defineConfig({
  plugins: [
    replaceCodePlugin({
      replacements: [{ from: "__API_URL__", to: JSON.stringify(API_URL) }],
    }),
  ],
});
