import { ToolchainPanel } from "@/components/toolchain/ToolchainPanel";

export function ToolchainPage() {
  return (
    <div className="flex flex-col gap-6 p-6 overflow-y-auto h-full">
      <div>
        <h1 className="text-xl font-semibold">Toolchain</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Verify and install required build dependencies.
        </p>
      </div>
      <ToolchainPanel />
    </div>
  );
}
