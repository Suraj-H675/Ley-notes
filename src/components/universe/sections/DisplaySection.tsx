import { useGraphSettings } from '@/hooks/useGraphSettings';
import { Slider } from '../Slider';
import type { GraphScope } from '@/types/graph-settings.types';

export interface DisplaySectionProps {
  scope: GraphScope;
}

export function DisplaySection({ scope }: DisplaySectionProps) {
  const { settings, update } = useGraphSettings(scope);
  if (!settings) return null;

  const set = (patch: Partial<typeof settings.display>) =>
    update({ ...settings, display: { ...settings.display, ...patch } });

  return (
    <div className="flex flex-col gap-3">
      <label className="flex items-center gap-2 text-[12.5px] text-foreground/85">
        <input
          type="checkbox"
          checked={settings.display.showLabels}
          onChange={(e) => set({ showLabels: e.target.checked })}
        />
        <span>Show labels</span>
      </label>
      <Slider
        label="Node size"
        value={settings.display.nodeSize}
        min={0.5}
        max={2.5}
        step={0.05}
        onChange={(v) => set({ nodeSize: v })}
      />
      <Slider
        label="Edge thickness"
        value={settings.display.edgeThickness}
        min={0.5}
        max={3}
        step={0.1}
        onChange={(v) => set({ edgeThickness: v })}
      />
      <Slider
        label="Text fade"
        value={settings.display.textFade}
        min={0}
        max={1}
        step={0.05}
        onChange={(v) => set({ textFade: v })}
        format={(v) => v.toFixed(2)}
      />
    </div>
  );
}
