import { useEffect, useState, useCallback, useRef } from "react";
import { useTourStore } from "../../stores/tourStore";

interface Rect {
  top: number;
  left: number;
  width: number;
  height: number;
}

export default function AppTour() {
  const { isActive, currentStep, steps, nextStep, prevStep, endTour } =
    useTourStore();

  const [targetRect, setTargetRect] = useState<Rect | null>(null);
  const tooltipRef = useRef<HTMLDivElement>(null);
  const [tooltipPos, setTooltipPos] = useState<{ top: number; left: number }>({
    top: 0,
    left: 0,
  });

  const step = steps[currentStep];
  const isFirst = currentStep === 0;
  const isLast = currentStep === steps.length - 1;
  const isCenterModal = !step?.target;

  // Find and track the target element
  const updateTargetRect = useCallback(() => {
    if (!step?.target) {
      setTargetRect(null);
      return;
    }
    const el = document.querySelector(step.target);
    if (el) {
      const rect = el.getBoundingClientRect();
      setTargetRect({
        top: rect.top,
        left: rect.left,
        width: rect.width,
        height: rect.height,
      });
    } else {
      setTargetRect(null);
    }
  }, [step?.target]);

  useEffect(() => {
    if (!isActive) return;
    updateTargetRect();
    const interval = setInterval(updateTargetRect, 300);
    window.addEventListener("resize", updateTargetRect);
    window.addEventListener("scroll", updateTargetRect, true);
    return () => {
      clearInterval(interval);
      window.removeEventListener("resize", updateTargetRect);
      window.removeEventListener("scroll", updateTargetRect, true);
    };
  }, [isActive, updateTargetRect]);

  // Calculate tooltip position
  useEffect(() => {
    if (!isActive || !step) return;

    if (isCenterModal) {
      setTooltipPos({
        top: window.innerHeight / 2,
        left: window.innerWidth / 2,
      });
      return;
    }

    if (!targetRect) return;

    const tooltipEl = tooltipRef.current;
    const tw = tooltipEl?.offsetWidth ?? 380;
    const th = tooltipEl?.offsetHeight ?? 200;
    const pad = 16;

    let top = 0;
    let left = 0;

    switch (step.position) {
      case "bottom":
        top = targetRect.top + targetRect.height + pad;
        left = targetRect.left + targetRect.width / 2 - tw / 2;
        break;
      case "top":
        top = targetRect.top - th - pad;
        left = targetRect.left + targetRect.width / 2 - tw / 2;
        break;
      case "right":
        top = targetRect.top + targetRect.height / 2 - th / 2;
        left = targetRect.left + targetRect.width + pad;
        break;
      case "left":
        top = targetRect.top + targetRect.height / 2 - th / 2;
        left = targetRect.left - tw - pad;
        break;
      default:
        top = targetRect.top + targetRect.height + pad;
        left = targetRect.left;
    }

    // Clamp to viewport
    top = Math.max(pad, Math.min(top, window.innerHeight - th - pad));
    left = Math.max(pad, Math.min(left, window.innerWidth - tw - pad));

    setTooltipPos({ top, left });
  }, [isActive, step, targetRect, isCenterModal]);

  // Keyboard navigation
  useEffect(() => {
    if (!isActive) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        endTour();
      } else if (e.key === "ArrowRight" || e.key === "Enter") {
        e.preventDefault();
        nextStep();
      } else if (e.key === "ArrowLeft") {
        e.preventDefault();
        prevStep();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [isActive, endTour, nextStep, prevStep]);

  if (!isActive || !step) return null;

  // Spotlight clip path for the overlay
  const spotlightPad = 8;
  const clipPath =
    targetRect && !isCenterModal
      ? `polygon(
          0% 0%, 0% 100%, 100% 100%, 100% 0%, 0% 0%,
          ${targetRect.left - spotlightPad}px ${targetRect.top - spotlightPad}px,
          ${targetRect.left - spotlightPad}px ${targetRect.top + targetRect.height + spotlightPad}px,
          ${targetRect.left + targetRect.width + spotlightPad}px ${targetRect.top + targetRect.height + spotlightPad}px,
          ${targetRect.left + targetRect.width + spotlightPad}px ${targetRect.top - spotlightPad}px,
          ${targetRect.left - spotlightPad}px ${targetRect.top - spotlightPad}px
        )`
      : undefined;

  return (
    <div className="fixed inset-0 z-[9999]" role="dialog" aria-modal="true">
      {/* Dark overlay with spotlight cutout */}
      <div
        className="absolute inset-0 bg-black/60 transition-all duration-300"
        style={clipPath ? { clipPath } : undefined}
        onClick={endTour}
      />

      {/* Spotlight border glow */}
      {targetRect && !isCenterModal && (
        <div
          className="pointer-events-none absolute rounded-lg border-2 border-blue-400 shadow-[0_0_20px_rgba(59,130,246,0.5)] transition-all duration-300"
          style={{
            top: targetRect.top - spotlightPad,
            left: targetRect.left - spotlightPad,
            width: targetRect.width + spotlightPad * 2,
            height: targetRect.height + spotlightPad * 2,
          }}
        />
      )}

      {/* Tooltip / Modal card */}
      <div
        ref={tooltipRef}
        className={`absolute z-[10000] transition-all duration-300 ${
          isCenterModal
            ? "left-1/2 top-1/2 w-[480px] -translate-x-1/2 -translate-y-1/2"
            : "w-[380px]"
        }`}
        style={
          isCenterModal
            ? undefined
            : { top: tooltipPos.top, left: tooltipPos.left }
        }
      >
        <div className="overflow-hidden rounded-xl border border-neutral-200 bg-white shadow-2xl dark:border-neutral-700 dark:bg-neutral-800">
          {/* Header with step counter */}
          <div className="flex items-center justify-between border-b border-neutral-100 bg-gradient-to-r from-blue-50 to-indigo-50 px-5 py-3 dark:border-neutral-700 dark:from-blue-950/30 dark:to-indigo-950/30">
            <div className="flex items-center gap-2">
              <div className="flex h-6 w-6 items-center justify-center rounded-full bg-blue-600 text-[10px] font-bold text-white">
                {currentStep + 1}
              </div>
              <span className="text-[11px] font-medium text-neutral-500 dark:text-neutral-400">
                of {steps.length}
              </span>
            </div>
            {/* Progress dots */}
            <div className="flex gap-1">
              {steps.map((_, i) => (
                <div
                  key={i}
                  className={`h-1.5 rounded-full transition-all ${
                    i === currentStep
                      ? "w-4 bg-blue-600"
                      : i < currentStep
                        ? "w-1.5 bg-blue-400"
                        : "w-1.5 bg-neutral-300 dark:bg-neutral-600"
                  }`}
                />
              ))}
            </div>
            <button
              onClick={endTour}
              className="rounded p-1 text-neutral-400 hover:bg-neutral-200 hover:text-neutral-600 dark:hover:bg-neutral-700 dark:hover:text-neutral-300"
              title="Close tour (Esc)"
            >
              <svg
                className="h-4 w-4"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                strokeWidth={2}
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
            </button>
          </div>

          {/* Content */}
          <div className="px-5 py-4">
            <h3 className="text-sm font-bold text-neutral-900 dark:text-neutral-100">
              {step.title}
            </h3>
            <div className="mt-2 whitespace-pre-line text-xs leading-relaxed text-neutral-600 dark:text-neutral-400">
              {step.description}
            </div>
          </div>

          {/* Footer with navigation */}
          <div className="flex items-center justify-between border-t border-neutral-100 bg-neutral-50 px-5 py-3 dark:border-neutral-700 dark:bg-neutral-800/50">
            <button
              onClick={endTour}
              className="text-[11px] font-medium text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-300"
            >
              Skip tour
            </button>
            <div className="flex gap-2">
              {!isFirst && (
                <button
                  onClick={prevStep}
                  className="rounded-lg border border-neutral-200 bg-white px-3 py-1.5 text-[11px] font-medium text-neutral-700 hover:bg-neutral-50 dark:border-neutral-600 dark:bg-neutral-700 dark:text-neutral-200 dark:hover:bg-neutral-600"
                >
                  Back
                </button>
              )}
              <button
                onClick={nextStep}
                className="rounded-lg bg-blue-600 px-4 py-1.5 text-[11px] font-medium text-white hover:bg-blue-700"
              >
                {isLast ? "Finish" : "Next"}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
