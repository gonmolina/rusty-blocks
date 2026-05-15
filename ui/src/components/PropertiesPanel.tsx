import React from 'react';
import type { Node } from 'reactflow';
import { RotateCw } from 'lucide-react';

interface PropertiesPanelProps {
  selectedNode: Node | null;
  onUpdateParams: (nodeId: string, newParams: any) => void;
  onUpdateRotation: (nodeId: string, rotation: number) => void;
}

export const PropertiesPanel: React.FC<PropertiesPanelProps> = ({ 
  selectedNode, 
  onUpdateParams,
  onUpdateRotation 
}) => {
  if (!selectedNode) {
    return (
      <div className="absolute top-4 right-4 z-10 bg-white/90 backdrop-blur-sm p-6 rounded-xl shadow-2xl border border-slate-200 w-80 h-[calc(100vh-2rem)] flex flex-col justify-center items-center text-slate-400 italic">
        Selecciona un bloque para editar sus propiedades
      </div>
    );
  }

  const { type, data, id } = selectedNode;
  const params = data.params || {};
  const rotation = data.rotation || 0;

  const handleInputChange = (key: string, value: any) => {
    onUpdateParams(id, { ...params, [key]: value });
  };

  const handleRotate = () => {
    const nextRotation = (rotation + 90) % 360;
    onUpdateRotation(id, nextRotation);
  };

  const renderFields = () => {
    switch (type) {
      case 'Gain':
        return (
          <div className="space-y-4">
            <div>
              <label className="block text-xs font-bold text-slate-500 uppercase mb-1">Ganancia (k)</label>
              <input 
                type="number" 
                value={params.k || 0} 
                onChange={(e) => handleInputChange('k', parseFloat(e.target.value))}
                className="w-full p-2 bg-slate-50 border border-slate-200 rounded-md font-mono text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
            <div>
              <label className="block text-xs font-bold text-slate-500 uppercase mb-1">Ancho (width)</label>
              <input 
                type="number" 
                value={params.width || 1} 
                onChange={(e) => handleInputChange('width', parseInt(e.target.value))}
                className="w-full p-2 bg-slate-50 border border-slate-200 rounded-md font-mono text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
          </div>
        );

      case 'Integrator':
        return (
          <div className="space-y-4">
            <div>
              <label className="block text-xs font-bold text-slate-500 uppercase mb-1">Condiciones Iniciales (ic)</label>
              <input 
                type="text" 
                value={JSON.stringify(params.ic || [0])} 
                onChange={(e) => {
                  try {
                    const val = JSON.parse(e.target.value);
                    if (Array.isArray(val)) handleInputChange('ic', val);
                  } catch (e) {}
                }}
                className="w-full p-2 bg-slate-50 border border-slate-200 rounded-md font-mono text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                placeholder="[0.0, 0.0]"
              />
            </div>
          </div>
        );

      case 'Constant':
        return (
          <div className="space-y-4">
            <div>
              <label className="block text-xs font-bold text-slate-500 uppercase mb-1">Valor</label>
              <input 
                type="text" 
                value={JSON.stringify(params.value || [0])} 
                onChange={(e) => {
                  try {
                    const val = JSON.parse(e.target.value);
                    if (Array.isArray(val)) handleInputChange('value', val);
                  } catch (e) {}
                }}
                className="w-full p-2 bg-slate-50 border border-slate-200 rounded-md font-mono text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
          </div>
        );

      case 'Sum':
        return (
          <div className="space-y-4">
            <div>
              <label className="block text-xs font-bold text-slate-500 uppercase mb-1">Signos</label>
              <input 
                type="text" 
                value={params.signs || '++'} 
                onChange={(e) => handleInputChange('signs', e.target.value)}
                className="w-full p-2 bg-slate-50 border border-slate-200 rounded-md font-mono text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
          </div>
        );

      default:
        return <div className="text-sm text-slate-500">Este bloque no tiene parámetros configurables.</div>;
    }
  };

  return (
    <div className="absolute top-4 right-4 z-10 bg-white/90 backdrop-blur-sm p-6 rounded-xl shadow-2xl border border-slate-200 w-80 h-[calc(100vh-2rem)] flex flex-col overflow-y-auto">
      <div className="flex items-center gap-2 mb-6">
        <div className="w-8 h-8 bg-blue-600 rounded-lg flex items-center justify-center text-white font-bold uppercase">
          {type?.[0]}
        </div>
        <div>
          <h2 className="text-lg font-bold text-slate-800">{type}</h2>
          <p className="text-[10px] text-slate-400 font-mono uppercase tracking-tighter">ID: {id}</p>
        </div>
      </div>

      <div className="flex-1">
        <h3 className="text-[10px] font-black text-slate-400 uppercase tracking-widest mb-4">Parámetros</h3>
        {renderFields()}
      </div>

      <div className="mt-6 pt-6 border-t border-slate-100">
        <h3 className="text-[10px] font-black text-slate-400 uppercase tracking-widest mb-4">Orientación</h3>
        <button 
          onClick={handleRotate}
          className="w-full flex items-center justify-center gap-2 py-3 bg-slate-900 text-white rounded-lg font-bold text-xs hover:bg-slate-800 transition-all shadow-md active:scale-95 uppercase tracking-widest"
        >
          <RotateCw size={16} /> Rotar 90°
        </button>
        <p className="text-[9px] text-slate-400 mt-2 text-center">Ángulo actual: {rotation}°</p>
      </div>
    </div>
  );
};
