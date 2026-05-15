import type { NodeProps } from 'reactflow';
import { BaseNode } from './BaseNode';
import { Activity, Gauge, Hash, PlusCircle, StepForward, ArrowRightLeft, LogIn, LogOut, Disc, Laptop } from 'lucide-react';

export const IntegratorNode = ({ id, data, selected }: NodeProps) => (
  <BaseNode id={id} title="Integrator" icon={<Activity size={14} />} selected={selected} rotation={data.rotation}>
    <div className="text-center">
      <span className="text-2xl font-serif italic text-slate-700">∫</span>
      <div className="text-[10px] text-slate-400 mt-1 uppercase font-bold tracking-tighter">IC: {JSON.stringify(data.params?.ic || [0])}</div>
    </div>
  </BaseNode>
);

export const GainNode = ({ id, data, selected }: NodeProps) => (
  <BaseNode id={id} title="Gain" icon={<Gauge size={14} />} selected={selected} rotation={data.rotation}>
    <div className="text-center py-2">
      <span className="text-lg font-black text-blue-600">×{data.params?.k || 1}</span>
    </div>
  </BaseNode>
);

export const ConstantNode = ({ id, data, selected }: NodeProps) => (
  <BaseNode id={id} title="Constant" icon={<Hash size={14} />} inputs={0} selected={selected} rotation={data.rotation}>
    <div className="text-center font-mono text-slate-700 font-bold bg-slate-50 p-2 rounded border border-slate-100 shadow-inner">
      {JSON.stringify(data.params?.value || [0])}
    </div>
  </BaseNode>
);

export const SumNode = ({ id, data, selected }: NodeProps) => {
  const signs = data.params?.signs || '++';
  return (
    <BaseNode 
      id={id}
      title="Sum" 
      icon={<PlusCircle size={14} />} 
      inputs={signs.length} 
      selected={selected}
      rotation={data.rotation}
    >
      <div className="flex justify-center gap-1.5 py-1">
        {signs.split('').map((s: string, i: number) => (
          <span key={i} className="text-xs font-black text-slate-400 border border-slate-200 w-5 h-5 flex items-center justify-center rounded bg-slate-50">{s}</span>
        ))}
      </div>
    </BaseNode>
  );
};

export const StepNode = ({ id, data, selected }: NodeProps) => (
  <BaseNode id={id} title="Step" icon={<StepForward size={14} />} inputs={0} selected={selected} rotation={data.rotation}>
    <div className="text-center space-y-1">
      <div className="h-4 w-full border-l-2 border-b-2 border-slate-400 relative">
         <div className="absolute bottom-0 left-1/2 w-1/2 h-3 border-l-2 border-t-2 border-blue-500" />
      </div>
      <div className="text-[9px] font-bold text-slate-500 uppercase tracking-tighter">T: {data.params?.step_time}</div>
    </div>
  </BaseNode>
);

export const MuxNode = ({ id, data, selected }: NodeProps) => {
  const widths = data.params?.input_widths || [1, 1];
  return (
    <BaseNode id={id} title="Mux" icon={<ArrowRightLeft size={14} />} inputs={widths.length} selected={selected} rotation={data.rotation}>
      <div className="w-2 h-16 bg-slate-800 mx-auto rounded-full shadow-inner" />
    </BaseNode>
  );
};

export const DemuxNode = ({ id, data, selected }: NodeProps) => {
  const widths = data.params?.output_widths || [1, 1];
  return (
    <BaseNode id={id} title="Demux" icon={<ArrowRightLeft size={14} />} outputs={widths.length} selected={selected} rotation={data.rotation}>
      <div className="w-2 h-16 bg-slate-800 mx-auto rounded-full shadow-inner" />
    </BaseNode>
  );
};

export const PortNode = ({ id, data, selected, type }: NodeProps) => (
  <BaseNode 
    id={id}
    title={type === 'InPort' ? 'InPort' : 'OutPort'} 
    icon={type === 'InPort' ? <LogIn size={14} /> : <LogOut size={14} />}
    inputs={type === 'InPort' ? 0 : 1}
    outputs={type === 'InPort' ? 1 : 0}
    selected={selected}
    rotation={data.rotation}
  >
    <div className={`text-center font-bold text-xs p-1 rounded ${type === 'InPort' ? 'bg-green-50 text-green-700' : 'bg-red-50 text-red-700'}`}>
      W: {data.params?.width || 1}
    </div>
  </BaseNode>
);

export const FileSinkNode = ({ id, data, selected }: NodeProps) => (
  <BaseNode id={id} title="FileSink" icon={<Disc size={14} />} outputs={0} selected={selected} rotation={data.rotation}>
    <div className="text-[10px] font-mono text-slate-600 truncate bg-slate-50 p-1.5 rounded border border-slate-100">
      {data.params?.filename || 'output.csv'}
    </div>
    <div className="text-[9px] text-center text-slate-400 mt-1 uppercase font-bold tracking-tighter">Int: {data.params?.interval}s</div>
  </BaseNode>
);

export const SubsystemNode = ({ id, data, selected }: NodeProps) => (
  <BaseNode id={id} title="Subsystem" icon={<Laptop size={14} />} selected={selected} rotation={data.rotation}>
    <div className="p-2 border-2 border-dashed border-slate-200 rounded-lg bg-slate-50 flex items-center justify-center min-h-[40px]">
      <span className="text-[10px] font-bold text-slate-400 uppercase tracking-widest text-center">
        {data.params?.name || 'Hierarchical'}
      </span>
    </div>
  </BaseNode>
);
