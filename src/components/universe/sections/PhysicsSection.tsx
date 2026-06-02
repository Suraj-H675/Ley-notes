import { useGraphSettings } from '@/hooks/useGraphSettings';
import { Slider } from '../Slider';
import type { GraphScope } from '@/types/graph-settings.types';

export interface PhysicsSectionProps {
  scope: GraphScope;
}

export function PhysicsSection({ scope }: PhysicsSectionProps) {
  const { settings, update } = useGraphSettings(scope);
  if (!settings) return null;

  const set = (patch: Partial<typeof settings.physics>) =>
    update({ ...settings, physics: { ...settings.physics, ...patch } });

  return (
    <div className="flex flex-col gap-3">
      <Slider
        label="Center force"
        value={settings.physics.centerForce}
        min={0}
        max={2}
        step={0.05}
        onChange={(v) => set({ centerForce: v })}
      />
      <Slider
        label="Charge force"
        value={settings.physics.chargeForce}
        min={-200}
        max={0}
        step={5}
        onChange={(v) => set({ chargeForce: v })}
        format={(v) => v.toFixed(0)}
      />
      <Slider
        label="Link force"
        value={settings.physics.linkForce}
        min={0}
        max={2}
        step={0.05}
        onChange={(v) => set({ linkForce: v })}
      />
      <Slider
        label="Link distance"
        value={settings.physics.linkDistance}
        min={20}
        max={200}
        step={5}
        onChange={(v) => set({ linkDistance: v })}
        format={(v) => v.toFixed(0)}
      />
    </div>
  );
}
