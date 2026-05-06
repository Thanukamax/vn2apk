// Re-export from PipelineContext so existing imports continue to work.
export type { BuildState, StageState } from "@/contexts/PipelineContext";
export { usePipelineContext as usePipeline } from "@/contexts/PipelineContext";
