import type { NodeProps } from 'reactflow';
import { BaseNode } from './BaseNode';
import { Activity, Gauge, Hash, PlusCircle, StepForward, ArrowRightLeft, LogIn, LogOut, Disc, Laptop } from 'lucide-react';

export const IntegratorNode = ({ id, data, selected }: NodeProps) => (
  <BaseNode id={id} title="Integrator" name={data.params?.name} icon={<Activity size={14} />} selected={selected} rotation={data.rotation} inputLabels={["u"]} outputLabels={["y"]}>
    <div className="text-center">
      <span className="text-2xl font-serif italic text-slate-700">∫</span>
      <div className="text-[10px] text-slate-400 mt-1 uppercase font-bold tracking-tighter">IC: {JSON.stringify(data.params?.ic || [0])}</div>
    </div>
  </BaseNode>
);

export const GainNode = ({ id, data, selected }: NodeProps) => (
  <BaseNode id={id} title="Gain" name={data.params?.name} icon={<Gauge size={14} />} selected={selected} rotation={data.rotation} inputLabels={["in"]} outputLabels={["out"]}>
    <div className="text-center py-2">
      <span className="text-lg font-black text-blue-600">×{data.params?.k || 1}</span>
    </div>
  </BaseNode>
);

export const ConstantNode = ({ id, data, selected }: NodeProps) => (
  <BaseNode id={id} title="Constant" name={data.params?.name} icon={<Hash size={14} />} inputs={0} selected={selected} rotation={data.rotation} outputLabels={["val"]}>
    <div className="text-center font-mono text-slate-700 font-bold bg-slate-50 p-2 rounded border border-slate-100 shadow-inner text-xs">
      {JSON.stringify(data.params?.value || [0])}
    </div>
  </BaseNode>
);

export const SumNode = ({ id, data, selected }: NodeProps) => {
  const signs = data.params?.signs || '++';
  return (
    <BaseNode id={id} title="Sum" name={data.params?.name} icon={<PlusCircle size={14} />} inputs={signs.length} selected={selected} rotation={data.rotation} outputLabels={["Σ"]}>
      <div className="flex justify-center gap-1.5 py-1">
        {signs.split('').map((s: string, i: number) => (
          <span key={i} className="text-[10px] font-black text-slate-500 border border-slate-200 w-5 h-5 flex items-center justify-center rounded bg-slate-50">{s}</span>
        ))}
      </div>
    </BaseNode>
  );
};

export const StepNode = ({ id, data, selected }: NodeProps) => (
  <BaseNode id={id} title="Step" name={data.params?.name} icon={<StepForward size={14} />} inputs={0} selected={selected} rotation={data.rotation} outputLabels={["out"]}>
    <div className="text-center space-y-1">
      <div className="h-4 w-full border-l-2 border-b-2 border-slate-400 relative">
         <div className="absolute bottom-0 left-1/2 w-1/2 h-3 border-l-2 border-t-2 border-blue-500" />
      </div>
      <div className="text-[9px] font-bold text-slate-500 uppercase tracking-tighter text-[8px]">T:{data.params?.step_time}</div>
    </div>
  </BaseNode>
);

export const MuxNode = ({ id, data, selected }: NodeProps) => {
  const widths = data.params?.input_widths || [1, 1];
  return (
    <BaseNode id={id} title="Mux" name={data.params?.name} icon={<ArrowRightLeft size={14} />} inputs={widths.length} selected={selected} rotation={data.rotation} outputLabels={["mix"]}>
      <div className="w-2 h-12 bg-slate-800 mx-auto rounded-full" />
    </BaseNode>
  );
};

export const DemuxNode = ({ id, data, selected }: NodeProps) => {
  const widths = data.params?.output_widths || [1, 1];
  return (
    <BaseNode id={id} title="Demux" name={data.params?.name} icon={<ArrowRightLeft size={14} />} outputs={widths.length} selected={selected} rotation={data.rotation} inputLabels={["mix"]}>
      <div className="w-2 h-12 bg-slate-800 mx-auto rounded-full" />
    </BaseNode>
  );
};

export const InPortNode = ({ id, data, selected }: NodeProps) => (
  <BaseNode id={id} title="InPort" name={data.params?.name} icon={<LogIn size={14} />} inputs={0} outputs={1} selected={selected} rotation={data.rotation} outputLabels={[`p${id.split('_').pop()}`]}>
    <div className="text-center font-black text-[10px] p-1 rounded bg-green-100 text-green-800 uppercase tracking-tighter">
      Input W:{data.params?.width || 1}
    </div>
  </BaseNode>
);

export const OutPortNode = ({ id, data, selected }: NodeProps) => (
  <BaseNode id={id} title="OutPort" name={data.params?.name} icon={<LogOut size={14} />} inputs={1} outputs={0} selected={selected} rotation={data.rotation} inputLabels={[`p${id.split('_').pop()}`]}>
    <div className="text-center font-black text-[10px] p-1 rounded bg-red-100 text-red-800 uppercase tracking-tighter">
      Output W:{data.params?.width || 1}
    </div>
  </BaseNode>
);

export const FileSinkNode = ({ id, data, selected }: NodeProps) => (
  <BaseNode id={id} title="FileSink" name={data.params?.name} icon={<Disc size={14} />} outputs={0} selected={selected} rotation={data.rotation} inputLabels={["save"]}>
    <div className="text-[9px] font-mono text-slate-600 truncate bg-slate-50 p-1 rounded border border-slate-200">
      {data.params?.filename || 'output.csv'}
    </div>
  </BaseNode>
);

export const SubsystemNode = ({ id, data, selected }: NodeProps) => (
  <BaseNode 
    id={id} title="Subsystem" name={data.params?.name} icon={<Laptop size={14} />} selected={selected} rotation={data.rotation}
    dynamicInputs={data.params?._numInputs}
    dynamicOutputs={data.params?._numOutputs}
  >
    <div className="p-3 border-2 border-dashed border-slate-300 rounded-lg bg-slate-50 flex items-center justify-center min-h-[50px]">
      <span className="text-[10px] font-black text-slate-400 uppercase tracking-widest text-center leading-tight">
        {data.params?.name || 'Hierarchical Block'}
      </span>
    </div>
  </BaseNode>
);
