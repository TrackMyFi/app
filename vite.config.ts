import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import ui from "@nuxt/ui/vite";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [
    vue(), 
    ui({
      ui: {
        accordion: {
          slots: {
            root: 'w-full u-accordion',
            trigger: 'group flex-1 flex items-center gap-1.5 font-medium text-sm px-3 py-3.5 outline-primary/25 focus-visible:outline-3 min-w-0 rounded-md',
            body: 'text-sm py-3.5 px-3',
          },
        },
        table: {
          slots: {
            td: 'px-4 py-2 text-sm text-default whitespace-nowrap [&:has([role=checkbox])]:pe-0',
            th: 'px-4 py-2 text-xs text-highlighted text-left rtl:text-right font-semibold border-b border-default [&:has([role=checkbox])]:pe-0',
            separator: 'hidden',
          },
        },
        modal: {
          slots: {
            content: 'bg-default divide-none flex flex-col focus:outline-none',
            body: 'flex-1 p-4 sm:p-6',
            header: 'flex items-center gap-1.5 p-4 pb-0 sm:px-6 sm:pt-6 min-h-0',
            footer: 'flex items-center gap-1.5 p-4 pt-0 sm:px-6 sm:pb-6',
          },
        },
        colors: {
          primary: 'emerald',
          neutral: 'mist'
        },
        button: {
          variants: {
            size: {
              md: { leadingIcon: 'size-4', trailingIcon: 'size-4' },
              lg: { leadingIcon: 'size-4', trailingIcon: 'size-4' },
            },
          },
        },
        input: {
          variants: {
            size: {
              md: { leadingIcon: 'size-4', trailingIcon: 'size-4' },
              lg: { leadingIcon: 'size-4', trailingIcon: 'size-4' },
            },
          },
        },
        icons: {
          arrowDown: 'i-ph-arrow-down',
          arrowLeft: 'i-ph-arrow-left',
          arrowRight: 'i-ph-arrow-right',
          arrowUp: 'i-ph-arrow-up',
          caution: 'i-ph-warning-circle',
          check: 'i-ph-check',
          chevronDoubleLeft: 'i-ph-caret-double-left',
          chevronDoubleRight: 'i-ph-caret-double-right',
          chevronDown: 'i-ph-caret-down',
          chevronLeft: 'i-ph-caret-left',
          chevronRight: 'i-ph-caret-right',
          chevronUp: 'i-ph-caret-up',
          close: 'i-ph-x',
          copy: 'i-ph-copy',
          copyCheck: 'i-ph-check-circle',
          dark: 'i-ph-moon',
          drag: 'i-ph-dots-six-vertical',
          ellipsis: 'i-ph-dots-three',
          error: 'i-ph-x-circle',
          external: 'i-ph-arrow-up-right',
          eye: 'i-ph-eye',
          eyeOff: 'i-ph-eye-slash',
          file: 'i-ph-file',
          folder: 'i-ph-folder',
          folderOpen: 'i-ph-folder-open',
          hash: 'i-ph-hash',
          info: 'i-ph-info',
          light: 'i-ph-sun',
          loading: 'i-ph-circle-notch',
          menu: 'i-ph-list',
          minus: 'i-ph-minus',
          panelClose: 'i-ph-caret-left',
          panelOpen: 'i-ph-caret-right',
          plus: 'i-ph-plus',
          reload: 'i-ph-arrow-counter-clockwise',
          search: 'i-ph-magnifying-glass',
          stop: 'i-ph-square',
          success: 'i-ph-check-circle',
          system: 'i-ph-monitor',
          tip: 'i-ph-lightbulb',
          upload: 'i-ph-upload',
          warning: 'i-ph-warning'
        }
      }
    })
  ],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
