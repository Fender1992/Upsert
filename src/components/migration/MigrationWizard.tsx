import { useCallback, useMemo } from "react";
import { useMigrationStore } from "../../stores/migrationStore";
import SelectSource from "./steps/SelectSource";
import SelectTarget from "./steps/SelectTarget";
import MapTables from "./steps/MapTables";
import ConfigureMode from "./steps/ConfigureMode";
import TransformStep from "./steps/TransformStep";
import DryRun from "./steps/DryRun";
import Execute from "./steps/Execute";

const STEPS = [
  { number: 1, label: "Source" },
  { number: 2, label: "Target" },
  { number: 3, label: "Map Tables" },
  { number: 4, label: "Configure" },
  { number: 5, label: "Transform" },
  { number: 6, label: "Dry Run" },
  { number: 7, label: "Execute" },
] as const;

interface Props {
  onClose: () => void;
}

export default function MigrationWizard({ onClose }: Props) {
  const {
    wizardStep,
    setWizardStep,
    sourceConnectionId,
    targetConnectionId,
    tableMappings,
    status,
    reset,
  } = useMigrationStore();

  const canGoNext = useMemo(() => {
    switch (wizardStep) {
      case 1:
        return sourceConnectionId !== null;
      case 2:
        return targetConnectionId !== null;
      case 3:
        return tableMappings.some((m) => m.included && m.targetTable);
      case 4:
        return true; // Config always valid with defaults
      case 5:
        return true; // Transforms are optional
      case 6:
        return true; // Dry run is optional
      case 7:
        return false; // Last step
      default:
        return false;
    }
  }, [wizardStep, sourceConnectionId, targetConnectionId, tableMappings]);

  const handleNext = useCallback(() => {
    if (canGoNext && wizardStep < 7) {
      setWizardStep(wizardStep + 1);
    }
  }, [canGoNext, wizardStep, setWizardStep]);

  const handleBack = useCallback(() => {
    if (wizardStep > 1) {
      setWizardStep(wizardStep - 1);
    }
  }, [wizardStep, setWizardStep]);

  const handleCancel = useCallback(() => {
    reset();
    onClose();
  }, [reset, onClose]);

  const isExecuting =
    status === "running" || status === "completed" || status === "cancelled";

  return (
    <div className="flex h-full flex-col overflow-hidden">
      {/* Horizontal stepper */}
      <div className="shrink-0 border-b border-neutral-200 bg-neutral-50 px-6 py-3 dark:border-neutral-700 dark:bg-neutral-850">
        <div className="flex items-center justify-between">
          {STEPS.map((step, idx) => {
            const isActive = wizardStep === step.number;
            const isCompleted = wizardStep > step.number;
            const isLast = idx === STEPS.length - 1;

            return (
              <div key={step.number} className="flex flex-1 items-center">
                {/* Step circle + label */}
                <div className="flex items-center gap-2">
                  <div
                    className={`flex h-6 w-6 shrink-0 items-center justify-center rounded-full text-[10px] font-bold transition-colors ${
                      isActive
                        ? "bg-blue-600 text-white"
                        : isCompleted
                          ? "bg-green-500 text-white"
                          : "bg-neutral-200 text-neutral-500 dark:bg-neutral-700 dark:text-neutral-400"
                    }`}
                  >
                    {isCompleted ? (
                      <svg
                        className="h-3.5 w-3.5"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                        strokeWidth={3}
                      >
                        <path
                          strokeLinecap="round"
                          strokeLinejoin="round"
                          d="M4.5 12.75l6 6 9-13.5"
                        />
                      </svg>
                    ) : (
                      step.number
                    )}
                  </div>
                  <span
                    className={`hidden text-[11px] font-medium sm:inline ${
                      isActive
                        ? "text-blue-600 dark:text-blue-400"
                        : isCompleted
                          ? "text-green-600 dark:text-green-400"
                          : "text-neutral-400 dark:text-neutral-500"
                    }`}
                  >
                    {step.label}
                  </span>
                </div>

                {/* Connector line */}
                {!isLast && (
                  <div
                    className={`mx-2 h-px flex-1 ${
                      isCompleted
                        ? "bg-green-400 dark:bg-green-600"
                        : "bg-neutral-200 dark:bg-neutral-700"
                    }`}
                  />
                )}
              </div>
            );
          })}
        </div>
      </div>

      {/* Step content */}
      <div className="flex-1 overflow-y-auto px-6 py-5">
        {wizardStep === 1 && <SelectSource />}
        {wizardStep === 2 && <SelectTarget />}
        {wizardStep === 3 && <MapTables />}
        {wizardStep === 4 && <ConfigureMode />}
        {wizardStep === 5 && <TransformStep />}
        {wizardStep === 6 && <DryRun />}
        {wizardStep === 7 && <Execute />}
      </div>

      {/* Navigation footer */}
      <div className="flex shrink-0 items-center justify-between border-t border-neutral-200 bg-neutral-50 px-6 py-3 dark:border-neutral-700 dark:bg-neutral-850">
        {/* Left: Cancel */}
        <button
          onClick={handleCancel}
          disabled={isExecuting}
          className="rounded border border-neutral-300 px-3 py-1.5 text-xs text-neutral-700 hover:bg-neutral-100 disabled:opacity-50 dark:border-neutral-600 dark:text-neutral-300 dark:hover:bg-neutral-700"
        >
          Cancel
        </button>

        {/* Right: Back / Next */}
        <div className="flex gap-2">
          <button
            onClick={handleBack}
            disabled={wizardStep === 1 || isExecuting}
            className="rounded border border-neutral-300 px-3 py-1.5 text-xs text-neutral-700 hover:bg-neutral-100 disabled:opacity-50 dark:border-neutral-600 dark:text-neutral-300 dark:hover:bg-neutral-700"
          >
            Back
          </button>
          {wizardStep < 7 && (
            <button
              onClick={handleNext}
              disabled={!canGoNext}
              className="rounded bg-blue-600 px-4 py-1.5 text-xs font-medium text-white hover:bg-blue-700 disabled:opacity-50"
            >
              Next
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
