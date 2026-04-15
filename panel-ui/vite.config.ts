import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";

const xavier2Target = process.env.XAVIER2_WEB_PROXY_TARGET ?? "http://127.0.0.1:8003";

export default defineConfig(({ command }) => {
  const isBuild = command === "build";

  return {
    base: isBuild ? "/panel/" : "/",
    plugins: [react()],
    resolve: {
      alias: {
        "@openuidev/react-headless": path.resolve(
          __dirname,
          "../node_modules/@openuidev/react-headless/dist/index.js",
        ),
        zustand: path.resolve(__dirname, "../node_modules/zustand"),
        "zustand/react/shallow": path.resolve(
          __dirname,
          "../node_modules/zustand/react/shallow.js",
        ),
      },
    },
    server: {
      host: "127.0.0.1",
      port: 4174,
      proxy: {
        "/health": {
          target: xavier2Target,
          changeOrigin: true,
        },
        "/panel/api": {
          target: xavier2Target,
          changeOrigin: true,
        },
      },
    },
    build: {
      outDir: "build",
      emptyOutDir: true,
      assetsDir: "assets",
      rollupOptions: {
        output: {
          entryFileNames: "assets/index.js",
          chunkFileNames: "assets/[name].js",
          assetFileNames: (assetInfo) => {
            if (assetInfo.name?.endsWith(".css")) {
              return "assets/index.css";
            }
            return "assets/[name][extname]";
          },
        },
      },
    },
  };
});
