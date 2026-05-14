import type { ReactNode } from 'react';
import { Handle, Position } from 'reactflow';

interface BaseNodeProps {
  title: string;
  icon?: ReactNode;
  inputs?: number;
  outputs?: number;
  selected?: boolean;
  children?: ReactNode;
}

export const BaseNode: React.FC<BaseNodeProps> = ({ 
  title, 
  icon, 
  inputs = 1, 
  outputs = 1, 
  selected,
  children 
}) => {
  return (
    <div className={`
      min-w-[120px] bg-white rounded-md border-2 transition-all shadow-sm
      ${selected ? 'border-blue-500 shadow-md ring-2 ring-blue-100' : 'border-slate-200'}
    `}>
      {/* Header */}
      <div className="bg-slate-50 border-b border-slate-100 px-3 py-1.5 rounded-t-md flex items-center gap-2">
        {icon && <span className="text-slate-500">{icon}</span>}
        <span className="text-xs font-bold text-slate-700 uppercase tracking-wider">{title}</span>
      </div>

      {/* Content */}
      <div className="p-3">
        {children}
      </div>

      {/* Input Handles */}
      <div className="absolute top-0 bottom-0 -left-1 flex flex-col justify-around py-8">
        {Array.from({ length: inputs }).map((_, i) => (
          <Handle
            key={`in-${i}`}
            type="target"
            position={Position.Left}
            id={`in-${i}`}
            className="w-3 h-3 bg-slate-400 border-2 border-white"
            style={{ top: inputs > 1 ? `${((i + 1) * 100) / (inputs + 1)}%` : '50%' }}
          />
        ))}
      </div>

      {/* Output Handles */}
      <div className="absolute top-0 bottom-0 -right-1 flex flex-col justify-around py-8">
        {Array.from({ length: outputs }).map((_, i) => (
          <Handle
            key={`out-${i}`}
            type="source"
            position={Position.Right}
            id={`out-${i}`}
            className="w-3 h-3 bg-blue-500 border-2 border-white"
            style={{ top: outputs > 1 ? `${((i + 1) * 100) / (outputs + 1)}%` : '50%' }}
          />
        ))}
      </div>
    </div>
  );
};
