import React, { useEffect } from 'react';
import type { ReactNode } from 'react';
import { Handle, Position, useUpdateNodeInternals } from 'reactflow';

interface BaseNodeProps {
  id?: string;
  title: string;
  icon?: ReactNode;
  inputs?: number;
  outputs?: number;
  selected?: boolean;
  rotation?: number; // 0, 90, 180, 270
  children?: ReactNode;
}

const PortArrow = () => (
  <svg 
    width="10" 
    height="10" 
    viewBox="0 0 10 10" 
    fill="none" 
    xmlns="http://www.w3.org/2000/svg"
    className="text-white pointer-events-none"
  >
    <path d="M2 2L8 5L2 8V2Z" fill="currentColor" />
  </svg>
);

// Función para calcular la posición real del puerto basada en la rotación
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
  id,
  title, 
  icon, 
  inputs = 1, 
  outputs = 1, 
  selected,
  rotation = 0,
  children 
}) => {
  const updateNodeInternals = useUpdateNodeInternals();

  useEffect(() => {
    if (id) {
      updateNodeInternals(id);
    }
  }, [rotation, inputs, outputs, id, updateNodeInternals]);

  const rotationClass = {
    0: 'rotate-0',
    90: 'rotate-90',
    180: 'rotate-180',
    270: 'rotate-270'
  }[rotation as 0 | 90 | 180 | 270] || 'rotate-0';

  const inputPos = getMappedPosition('left', rotation);
  const outputPos = getMappedPosition('right', rotation);

  return (
    <div className="relative group">
      {/* Visual Box - Only this part rotates visually */}
      <div className={`
        min-w-[140px] bg-white rounded-md border-2 transition-all shadow-lg
        ${selected ? 'border-blue-500 ring-4 ring-blue-500/20' : 'border-slate-400'}
        ${rotationClass}
      `}>
        {/* Header */}
        <div className="bg-slate-100 border-b border-slate-200 px-3 py-2 rounded-t-md flex items-center gap-2">
          {icon && <div className="text-slate-600">{icon}</div>}
          <span className="text-[11px] font-black text-slate-800 uppercase tracking-tighter">{title}</span>
        </div>

        {/* Content Area */}
        <div className="p-4 bg-white min-h-[60px] flex items-center justify-center">
          {children}
        </div>
      </div>

      {/* Logic Handles - These stay outside the rotation but change their 'Position' prop */}
      {/* This ensures React Flow can calculate edge paths correctly */}
      
      <div key={`in-group-${rotation}-${inputs}`}>
        {Array.from({ length: inputs }).map((_, i) => (
          <Handle
            key={`in-${i}`}
            type="target"
            position={inputPos}
            id={`in-${i}`}
            className="!w-6 !h-6 !bg-slate-900 border-2 border-slate-400 flex items-center justify-center shadow-md hover:!bg-blue-600 transition-all"
            style={{ 
              zIndex: 20,
              // Ajustamos el offset dependiendo de si es horizontal o vertical
              [inputPos === Position.Left || inputPos === Position.Right ? 'top' : 'left']: 
                inputs > 1 ? `${((i + 1) * 100) / (inputs + 1)}%` : '50%',
              [inputPos]: '-12px',
              borderRadius: '4px',
              transform: (inputPos === Position.Left || inputPos === Position.Right) ? 'translateY(-50%)' : 'translateX(-50%)'
            }}
          >
            <div style={{ transform: `rotate(${-rotation}deg)` }}>
               <PortArrow />
            </div>
          </Handle>
        ))}
      </div>

      <div key={`out-group-${rotation}-${outputs}`}>
        {Array.from({ length: outputs }).map((_, i) => (
          <Handle
            key={`out-${i}`}
            type="source"
            position={outputPos}
            id={`out-${i}`}
            className="!w-6 !h-6 !bg-blue-700 border-2 border-blue-400 flex items-center justify-center shadow-md hover:!bg-blue-500 transition-all"
            style={{ 
              zIndex: 20,
              [outputPos === Position.Left || outputPos === Position.Right ? 'top' : 'left']: 
                outputs > 1 ? `${((i + 1) * 100) / (outputs + 1)}%` : '50%',
              [outputPos]: '-12px',
              borderRadius: '4px',
              transform: (outputPos === Position.Left || outputPos === Position.Right) ? 'translateY(-50%)' : 'translateX(-50%)'
            }}
          >
            <div style={{ transform: `rotate(${-rotation}deg)` }}>
              <PortArrow />
            </div>
          </Handle>
        ))}
      </div>
    </div>
  );
};
