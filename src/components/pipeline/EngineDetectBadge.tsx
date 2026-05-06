import { AlertTriangle, CheckCircle, HelpCircle } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { EngineDetectResult, EngineKind } from "@/types/pipeline";

const ENGINE_LABELS: Record<EngineKind, string> = {
  RpgMakerMV: "RPG Maker MV",
  RpgMakerMZ: "RPG Maker MZ",
  TyranoBuilder: "TyranoBuilder",
  RenPy: "Ren'Py",
  NScripter: "NScripter",
  Kirikiri: "KiriKiri",
  Unknown: "Unknown Engine",
};

const ENGINE_SUPPORTED: Record<EngineKind, boolean> = {
  RpgMakerMV: true,
  RpgMakerMZ: true,
  TyranoBuilder: true,
  RenPy: true,
  NScripter: false,
  Kirikiri: false,
  Unknown: false,
};

interface Props {
  result: EngineDetectResult;
}

export function EngineDetectBadge({ result }: Props) {
  const supported = ENGINE_SUPPORTED[result.engine];
  const label = ENGINE_LABELS[result.engine];

  return (
    <div className="flex items-center gap-3 rounded-lg border border-border bg-card px-4 py-3">
      {supported ? (
        <CheckCircle className="h-5 w-5 text-emerald-500 shrink-0" />
      ) : result.engine === "Unknown" ? (
        <HelpCircle className="h-5 w-5 text-muted-foreground shrink-0" />
      ) : (
        <AlertTriangle className="h-5 w-5 text-amber-500 shrink-0" />
      )}
      <div className="flex-1">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">{label}</span>
          <Badge variant={supported ? "success" : "warning"}>
            {supported ? "Supported" : "Not yet supported"}
          </Badge>
          <span className="text-xs text-muted-foreground">{result.confidence}% confidence</span>
        </div>
        <p className="text-xs text-muted-foreground mt-0.5">{result.details}</p>
      </div>
    </div>
  );
}
