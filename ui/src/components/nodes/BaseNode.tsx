import React, { useEffect } from 'react';
import { Handle, Position, useUpdateNodeInternals, useStore } from 'reactflow';
import type { ReactNode } from 'react';
import type { ReactFlowState } from 'reactflow';

interface BaseNodeProps {
  id?: string;
  title: string;
  name?: string; // Nombre personalizado del bloque
  icon?: ReactNode;
  inputs?: number;
  outputs?: number;
  selected?: boolean;
  rotation?: number;
  children?: ReactNode;
  dynamicInputs?: number;
  dynamicOutputs?: number;
  // Labels para los puertos
  inputLabels?: string[];
  outputLabels?: string[];
}

const PortArrow = () => (
  <svg width="8" height="8" viewBox="0 0 10 10" fill="none" className="text-white pointer-events-none">
    <path d="M2 2L8 5L2 8V2Z" fill="currentColor" />
  </svg>
);

const getMappedPosition = (base: 'left' | 'right', rotation: number): Position => {
  const angle = (rotation % 360 + 360) % 360;
  if (base === 'left') {
    if (angle === 0) return Position.Left;
    if (angle === 90) return Position.Top;
    if (angle === 180) return Position.Right;
    if (angle === 270) return Position.Bottom;
  } else {
    if (angle === 0) return Position.Right;
    if (angle === 90) return Position.Bottom;
    if (angle === 180) return Position.Left;
    if (angle === 270) return Position.Top;
  }
  return base === 'left' ? Position.Left : Position.Right;
};

export const BaseNode: React.FC<BaseNodeProps> = ({ 
  id, title, name, icon, inputs = 1, outputs = 1, selected, rotation = 0, children,
  dynamicInputs, dynamicOutputs, inputLabels = [], outputLabels = []
}) => {
  const updateNodeInternals = useUpdateNodeInternals();
  const edges = useStore((s: ReactFlowState) => s.edges);

  const finalInputs = dynamicInputs !== undefined ? dynamicInputs : inputs;
  const finalOutputs = dynamicOutputs !== undefined ? dynamicOutputs : outputs;

  useEffect(() => {
    if (id) updateNodeInternals(id);
  }, [rotation, finalInputs, finalOutputs, id, updateNodeInternals]);

  const rotationClass = { 0: 'rotate-0', 90: 'rotate-90', 180: 'rotate-180', 270: 'rotate-270' }[rotation as 0|90|180|270] || 'rotate-0';
  const inputPos = getMappedPosition('left', rotation);
  const outputPos = getMappedPosition('right', rotation);

  return (
    <div className="relative group font-sans">
      {/* Label de nombre sobre el bloque */}
      {name && (
        <div className="absolute -top-6 left-0 right-0 text-center">
          <span className="text-[10px] font-black text-slate-800 bg-white/80 px-2 py-0.5 rounded shadow-sm border border-slate-200 uppercase tracking-tight">
            {name}
          </span>
        </div>
      )}

      <div className={`min-w-[140px] bg-white rounded-md border-2 transition-all shadow-lg ${selected ? 'border-blue-600 ring-4 ring-blue-600/10' : 'border-slate-400'} ${rotationClass}`}>
        <div className="bg-slate-100 border-b border-slate-200 px-3 py-2 rounded-t-md flex items-center gap-2">
          {icon && <div className="text-slate-600">{icon}</div>}
          <span className="text-[9px] font-black text-slate-500 uppercase tracking-widest">{title}</span>
        </div>
        <div className="p-4 bg-white min-h-[60px] flex items-center justify-center">
          {children}
        </div>
      </div>

      {/* Input Handles */}
      <div key={`in-group-${rotation}-${finalInputs}`}>
        {Array.from({ length: finalInputs }).map((_, i) => {
          const handleId = `in-${i}`;
          const isConnected = edges.some(e => e.target === id && e.targetHandle === handleId);
          return (
            <div key={handleId} className="absolute pointer-events-none" style={{ 
              top: finalInputs > 1 ? `${((i + 1) * 100) / (finalInputs + 1)}%` : '50%',
              [inputPos]: '-12px', transform: (inputPos === Position.Left || inputPos === Position.Right) ? 'translateY(-50%)' : 'translateX(-50%)'
            }}>
              <Handle
                type="target" position={inputPos} id={handleId} isConnectable={!isConnected}
                className={`!relative !left-0 !top-0 !translate-y-0 !translate-x-0 !w-6 !h-6 border-2 flex items-center justify-center shadow-md transition-all pointer-events-auto ${isConnected ? '!bg-slate-400 border-slate-500 opacity-50 cursor-not-allowed' : '!bg-slate-900 border-slate-400 hover:!bg-blue-600'}`}
                style={{ borderRadius: '4px' }}
              >
                <div style={{ transform: `rotate(${-rotation}deg)` }}><PortArrow /></div>
              </Handle>
              {/* Etiqueta del puerto */}
              {inputLabels[i] && (
                <div className={`absolute whitespace-nowrap text-[8px] font-bold text-slate-400 uppercase tracking-tighter
                  ${inputPos === Position.Left ? 'left-8' : inputPos === Position.Right ? 'right-8' : inputPos === Position.Top ? 'top-8' : 'bottom-8'}`}>
                  {inputLabels[i]}
                </div>
              )}
            </div>
          );
        })}
      </div>

      {/* Output Handles */}
      <div key={`out-group-${rotation}-${finalOutputs}`}>
        {Array.from({ length: finalOutputs }).map((_, i) => {
          const handleId = `out-${i}`;
          return (
            <div key={handleId} className="absolute pointer-events-none" style={{ 
              top: finalOutputs > 1 ? `${((i + 1) * 100) / (finalOutputs + 1)}%` : '50%',
              [outputPos]: '-12px', transform: (outputPos === Position.Left || outputPos === Position.Right) ? 'translateY(-50%)' : 'translateX(-50%)'
            }}>
              <Handle
                type="source" position={outputPos} id={handleId}
                className="!relative !left-0 !top-0 !translate-y-0 !translate-x-0 !w-6 !h-6 !bg-blue-700 border-2 border-blue-400 flex items-center justify-center shadow-md hover:!bg-blue-500 transition-all pointer-events-auto"
                style={{ borderRadius: '4px' }}
              >
                <div style={{ transform: `rotate(${-rotation}deg)` }}><PortArrow /></div>
              </Handle>
              {/* Etiqueta del puerto */}
              {outputLabels[i] && (
                <div className={`absolute whitespace-nowrap text-[8px] font-bold text-blue-400 uppercase tracking-tighter
                  ${outputPos === Position.Right ? 'right-8' : outputPos === Position.Left ? 'left-8' : outputPos === Position.Bottom ? 'bottom-8' : 'top-8'}`}>
                  {outputLabels[i]}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
};
