import React, { useRef } from 'react';
import { Activity, Gauge, Hash, PlusCircle, Laptop, ArrowRightLeft, StepForward, Disc, LogIn, LogOut, Save, FolderOpen, Settings2, Play } from 'lucide-react';

const blocks = [
  { type: 'Constant', label: 'Constant', icon: <Hash size={16} />, category: 'Sources' },
  { type: 'Step', label: 'Step', icon: <StepForward size={16} />, category: 'Sources' },
  { type: 'Gain', label: 'Gain', icon: <Gauge size={16} />, category: 'Math' },
  { type: 'Sum', label: 'Sum', icon: <PlusCircle size={16} />, category: 'Math' },
  { type: 'Integrator', label: 'Integrator', icon: <Activity size={16} />, category: 'Continuous' },
  { type: 'Mux', label: 'Mux', icon: <ArrowRightLeft size={16} />, category: 'Routing' },
  { type: 'Demux', label: 'Demux', icon: <ArrowRightLeft size={16} />, category: 'Routing' },
  { type: 'InPort', label: 'InPort', icon: <LogIn size={16} />, category: 'Ports' },
  { type: 'OutPort', label: 'OutPort', icon: <LogOut size={16} />, category: 'Ports' },
  { type: 'FileSink', label: 'FileSink', icon: <Disc size={16} />, category: 'Sinks' },
  { type: 'Subsystem', label: 'Subsystem', icon: <Laptop size={16} />, category: 'Hierarchy' },
];

interface SidebarProps {
  onSave: () => void;
  onLoad: (file: File) => void;
  onSimulate: () => void;
  onOpenSettings: () => void;
  solverType: string;
}

export const Sidebar: React.FC<SidebarProps> = ({ onSave, onLoad, onSimulate, onOpenSettings, solverType }) => {
  const fileInputRef = useRef<HTMLInputElement>(null);

  const onDragStart = (event: React.DragEvent, nodeType: string) => {
    event.dataTransfer.setData('application/reactflow', nodeType);
    event.dataTransfer.effectAllowed = 'move';
  };

  const groupedBlocks = blocks.reduce((acc, block) => {
    (acc[block.category] = acc[block.category] || []).push(block);
    return acc;
  }, {} as Record<string, typeof blocks>);

  const handleFileChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (file) onLoad(file);
  };

  return (
    <aside className="absolute top-4 left-4 z-10 w-60 bg-white/90 backdrop-blur-sm rounded-xl shadow-2xl border border-slate-200 flex flex-col max-h-[calc(100vh-2rem)] overflow-hidden">
      <div className="p-4 border-b border-slate-100 bg-slate-50/50">
        <h1 className="text-xl font-black text-slate-900 tracking-tighter italic flex items-center gap-2">
          RUSTY-BLOCKS <span className="text-blue-600 text-xs not-italic font-bold tracking-tight ml-1">V2.0</span>
        </h1>
        <p className="text-[9px] text-slate-400 font-bold uppercase tracking-widest mt-1">Librería de Componentes</p>
      </div>

      <div className="flex-1 overflow-y-auto p-3 space-y-5 custom-scrollbar font-sans">
        {Object.entries(groupedBlocks).map(([category, items]) => (
          <div key={category}>
            <h3 className="text-[9px] font-black text-slate-400 uppercase tracking-[0.2em] mb-2 px-1">{category}</h3>
            <div className="grid grid-cols-1 gap-1.5">
              {items.map((block) => (
                <div
                  key={block.type}
                  className="flex items-center gap-3 p-2.5 bg-white border border-slate-100 rounded-lg cursor-grab hover:border-blue-300 hover:shadow-sm hover:bg-blue-50/30 transition-all group active:cursor-grabbing"
                  onDragStart={(event) => onDragStart(event, block.type)}
                  draggable
                >
                  <div className="text-slate-400 group-hover:text-blue-500 transition-colors">
                    {block.icon}
                  </div>
                  <span className="text-xs font-bold text-slate-600 group-hover:text-slate-900">{block.label}</span>
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>

      <div className="p-3 bg-white border-t border-slate-100 space-y-2">
        <div className="grid grid-cols-2 gap-2 mb-2">
          <button 
            onClick={onSave}
            className="flex items-center justify-center gap-2 py-2 bg-slate-100 text-slate-700 rounded-lg font-bold text-[9px] hover:bg-slate-200 transition-all uppercase tracking-wider"
          >
            <Save size={12} /> Guardar
          </button>
          <button 
            onClick={() => fileInputRef.current?.click()}
            className="flex items-center justify-center gap-2 py-2 bg-slate-100 text-slate-700 rounded-lg font-bold text-[9px] hover:bg-slate-200 transition-all uppercase tracking-wider"
          >
            <FolderOpen size={12} /> Cargar
          </button>
        </div>
        
        <input type="file" ref={fileInputRef} onChange={handleFileChange} className="hidden" accept=".json" />
        
        <div className="flex gap-2">
          <button 
            onClick={onOpenSettings}
            className="p-2.5 bg-slate-100 text-slate-600 rounded-lg hover:bg-slate-200 transition-all"
            title="Ajustes de Simulación"
          >
            <Settings2 size={18} />
          </button>
          <button 
            className="flex-1 flex items-center justify-center gap-2 py-2.5 bg-blue-600 text-white rounded-lg font-black text-xs hover:bg-blue-700 transition-all shadow-md active:scale-95 uppercase tracking-widest"
            onClick={onSimulate}
          >
            <Play size={14} fill="currentColor" /> Simular
          </button>
        </div>
        
        <div className="flex justify-between items-center px-1">
          <span className="text-[8px] font-bold text-slate-300 uppercase tracking-tighter">Solver actual:</span>
          <span className="text-[8px] font-bold text-blue-400 uppercase tracking-tighter">{solverType}</span>
        </div>
      </div>
    </aside>
  );
};
