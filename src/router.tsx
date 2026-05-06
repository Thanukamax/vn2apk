import { createHashRouter } from "react-router-dom";
import { Layout } from "@/components/layout/Layout";
import { ToolchainPage } from "@/pages/ToolchainPage";
import { BuildPage } from "@/pages/BuildPage";
import { SettingsPage } from "@/pages/SettingsPage";

export const router = createHashRouter([
  {
    path: "/",
    element: <Layout />,
    children: [
      { index: true, element: <ToolchainPage /> },
      { path: "toolchain", element: <ToolchainPage /> },
      { path: "build", element: <BuildPage /> },
      { path: "settings", element: <SettingsPage /> },
    ],
  },
]);
