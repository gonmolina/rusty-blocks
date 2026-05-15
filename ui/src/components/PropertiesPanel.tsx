import React from 'react';
import type { Node } from 'reactflow';
import { RotateCw, Type } from 'lucide-react';

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
    // Add common "Name" field for all blocks
    const nameField = (
      <div className="mb-6 pb-6 border-b border-slate-100">
        <label className="flex items-center gap-2 text-[10px] font-black text-blue-600 uppercase tracking-widest mb-2">
          <Type size={12} /> Nombre del Bloque
        </label>
        <input 
          type="text" 
          value={params.name || ''} 
          placeholder={`Ej: ${type} Principal`}
          onChange={(e) => handleInputChange('name', e.target.value)}
          className="w-full p-2.5 bg-blue-50/30 border border-blue-100 rounded-lg text-sm font-bold text-slate-700 focus:ring-2 focus:ring-blue-500 outline-none"
        />
      </div>
    );

    let specificFields = null;
    switch (type) {
      case 'Gain':
        specificFields = (
          <div className="space-y-4">
            <div>
              <label className="block text-xs font-bold text-slate-500 uppercase mb-1">Ganancia (k)</label>
              <input type="number" value={params.k || 0} onChange={(e) => handleInputChange('k', parseFloat(e.target.value))} className="w-full p-2 bg-slate-50 border border-slate-200 rounded-md font-mono text-sm" />
            </div>
            <div>
              <label className="block text-xs font-bold text-slate-500 uppercase mb-1">Ancho (width)</label>
              <input type="number" value={params.width || 1} onChange={(e) => handleInputChange('width', parseInt(e.target.value))} className="w-full p-2 bg-slate-50 border border-slate-200 rounded-md font-mono text-sm" />
            </div>
          </div>
        );
        break;

      case 'Integrator':
        specificFields = (
          <div className="space-y-4">
            <div>
              <label className="block text-xs font-bold text-slate-500 uppercase mb-1">Condiciones Iniciales (ic)</label>
              <input type="text" value={JSON.stringify(params.ic || [0])} onChange={(e) => { try { const val = JSON.parse(e.target.value); if (Array.isArray(val)) handleInputChange('ic', val); } catch (e) {} }} className="w-full p-2 bg-slate-50 border border-slate-200 rounded-md font-mono text-sm" />
            </div>
          </div>
        );
        break;

      case 'Constant':
        specificFields = (
          <div className="space-y-4">
            <div>
              <label className="block text-xs font-bold text-slate-500 uppercase mb-1">Valor</label>
              <input type="text" value={JSON.stringify(params.value || [0])} onChange={(e) => { try { const val = JSON.parse(e.target.value); if (Array.isArray(val)) handleInputChange('value', val); } catch (e) {} }} className="w-full p-2 bg-slate-50 border border-slate-200 rounded-md font-mono text-sm" />
            </div>
          </div>
        );
        break;

      case 'Sum':
        specificFields = (
          <div className="space-y-4">
            <div>
              <label className="block text-xs font-bold text-slate-500 uppercase mb-1">Signos</label>
              <input type="text" value={params.signs || '++'} onChange={(e) => handleInputChange('signs', e.target.value)} className="w-full p-2 bg-slate-50 border border-slate-200 rounded-md font-mono text-sm" />
            </div>
          </div>
        );
        break;

      case 'Step':
        specificFields = (
          <div className="space-y-4">
            <div>
              <label className="block text-xs font-bold text-slate-500 uppercase mb-1">Valor Inicial</label>
              <input type="number" value={params.initial_value} onChange={(e) => handleInputChange('initial_value', parseFloat(e.target.value))} className="w-full p-2 bg-slate-50 border border-slate-200 rounded-md font-mono text-sm" />
            </div>
            <div>
              <label className="block text-xs font-bold text-slate-500 uppercase mb-1">Valor Final</label>
              <input type="number" value={params.final_value} onChange={(e) => handleInputChange('final_value', parseFloat(e.target.value))} className="w-full p-2 bg-slate-50 border border-slate-200 rounded-md font-mono text-sm" />
            </div>
            <div>
              <label className="block text-xs font-bold text-slate-500 uppercase mb-1">Tiempo de Escalón</label>
              <input type="number" value={params.step_time} onChange={(e) => handleInputChange('step_time', parseFloat(e.target.value))} className="w-full p-2 bg-slate-50 border border-slate-200 rounded-md font-mono text-sm" />
            </div>
          </div>
        );
        break;

      case 'Mux':
        specificFields = (
          <div className="space-y-4">
            <div>
              <label className="block text-xs font-bold text-slate-500 uppercase mb-1">Anchos de Entrada</label>
              <input type="text" value={JSON.stringify(params.input_widths)} onChange={(e) => { try { const val = JSON.parse(e.target.value); if (Array.isArray(val)) handleInputChange('input_widths', val); } catch (e) {} }} className="w-full p-2 bg-slate-50 border border-slate-200 rounded-md font-mono text-sm" />
            </div>
          </div>
        );
        break;

      case 'Demux':
        specificFields = (
          <div className="space-y-4">
            <div>
              <label className="block text-xs font-bold text-slate-500 uppercase mb-1">Anchos de Salida</label>
              <input type="text" value={JSON.stringify(params.output_widths)} onChange={(e) => { try { const val = JSON.parse(e.target.value); if (Array.isArray(val)) handleInputChange('output_widths', val); } catch (e) {} }} className="w-full p-2 bg-slate-50 border border-slate-200 rounded-md font-mono text-sm" />
            </div>
          </div>
        );
        break;

      case 'InPort':
      case 'OutPort':
        specificFields = (
          <div className="space-y-4">
            <div>
              <label className="block text-xs font-bold text-slate-500 uppercase mb-1">Ancho (width)</label>
              <input type="number" value={params.width} onChange={(e) => handleInputChange('width', parseInt(e.target.value))} className="w-full p-2 bg-slate-50 border border-slate-200 rounded-md font-mono text-sm" />
            </div>
          </div>
        );
        break;

      case 'FileSink':
        specificFields = (
          <div className="space-y-4">
            <div>
              <label className="block text-xs font-bold text-slate-500 uppercase mb-1">Nombre de Archivo</label>
              <input type="text" value={params.filename} onChange={(e) => handleInputChange('filename', e.target.value)} className="w-full p-2 bg-slate-50 border border-slate-200 rounded-md font-mono text-sm" />
            </div>
            <div>
              <label className="block text-xs font-bold text-slate-500 uppercase mb-1">Intervalo (s)</label>
              <input type="number" value={params.interval} onChange={(e) => handleInputChange('interval', parseFloat(e.target.value))} className="w-full p-2 bg-slate-50 border border-slate-200 rounded-md font-mono text-sm" />
            </div>
          </div>
        );
        break;

      case 'Subsystem':
        specificFields = (
          <div className="space-y-4">
            <div>
              <label className="block text-xs font-bold text-slate-500 uppercase mb-1">Nombre</label>
              <input type="text" value={params.name} onChange={(e) => handleInputChange('name', e.target.value)} className="w-full p-2 bg-slate-50 border border-slate-200 rounded-md font-mono text-sm" />
            </div>
          </div>
        );
        break;

      default:
        specificFields = <div className="text-sm text-slate-500">Sin parámetros específicos.</div>;
    }

    return (
      <>
        {nameField}
        {specificFields}
      </>
    );
  };

  return (
    <div className="absolute top-4 right-4 z-10 bg-white/90 backdrop-blur-sm p-6 rounded-xl shadow-2xl border border-slate-200 w-80 h-[calc(100vh-2rem)] flex flex-col overflow-y-auto font-sans">
      <div className="flex items-center gap-3 mb-6">
        <div className="w-10 h-10 bg-slate-900 rounded-xl flex items-center justify-center text-white font-black uppercase shadow-lg border-2 border-slate-700">
          {type?.[0]}
        </div>
        <div>
          <h2 className="text-lg font-black text-slate-800 tracking-tight leading-none">{type}</h2>
          <p className="text-[9px] text-slate-400 font-mono uppercase mt-1">ID: {id}</p>
        </div>
      </div>

      <div className="flex-1">
        {renderFields()}
      </div>

      <div className="mt-6 pt-6 border-t border-slate-100 bg-slate-50/50 -mx-6 px-6 -mb-6 pb-6 rounded-b-xl">
        <h3 className="text-[10px] font-black text-slate-400 uppercase tracking-widest mb-4">Orientación</h3>
        <button 
          onClick={handleRotate}
          className="w-full flex items-center justify-center gap-2 py-3 bg-slate-900 text-white rounded-lg font-bold text-xs hover:bg-slate-800 transition-all shadow-md active:scale-95 uppercase tracking-widest"
        >
          <RotateCw size={16} /> Rotar 90°
        </button>
        <p className="text-[9px] text-slate-400 mt-2 text-center font-bold tracking-wider">ÁNGULO ACTUAL: {rotation}°</p>
      </div>
    </div>
  );
};
