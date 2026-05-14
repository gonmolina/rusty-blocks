import type { NodeProps } from 'reactflow';
import { BaseNode } from './BaseNode';
import { Activity, Gauge, Hash, PlusCircle } from 'lucide-react';

export const IntegratorNode = ({ data, selected }: NodeProps) => (
  <BaseNode title="Integrator" icon={<Activity size={14} />} selected={selected}>
    <div className="text-center">
      <span className="text-2xl font-serif">∫</span>
      <div className="text-[10px] text-slate-400 mt-1">IC: {JSON.stringify(data.params?.ic || [0])}</div>
    </div>
  </BaseNode>
);

export const GainNode = ({ data, selected }: NodeProps) => (
  <BaseNode title="Gain" icon={<Gauge size={14} />} selected={selected}>
    <div className="text-center">
      <span className="text-lg font-bold">× {data.params?.k || 1}</span>
    </div>
  </BaseNode>
);

export const ConstantNode = ({ data, selected }: NodeProps) => (
  <BaseNode title="Constant" icon={<Hash size={14} />} inputs={0} selected={selected}>
    <div className="text-center font-mono text-blue-600 font-bold">
      {JSON.stringify(data.params?.value || [0])}
    </div>
  </BaseNode>
);

export const SumNode = ({ data, selected }: NodeProps) => {
  const signs = data.params?.signs || '++';
  return (
    <BaseNode 
      title="Sum" 
      icon={<PlusCircle size={14} />} 
      inputs={signs.length} 
      selected={selected}
    >
      <div className="flex justify-center gap-1">
        {signs.split('').map((s: string, i: number) => (
          <span key={i} className="text-xs font-bold text-slate-500">{s}</span>
        ))}
      </div>
    </BaseNode>
  );
};
