import React from 'react';
import { Settings, X } from 'lucide-react';

export interface SimulationParams {
  dt: number;
  t_final: number;
  solver: 'Euler' | 'RK4' | 'RK45';
  atol: number;
  rtol: number;
}

interface SimulationSettingsProps {
  isOpen: boolean;
  onClose: () => void;
  params: SimulationParams;
  onUpdate: (params: SimulationParams) => void;
}

export const SimulationSettings: React.FC<SimulationSettingsProps> = ({ 
  isOpen, 
  onClose, 
  params, 
  onUpdate 
}) => {
  if (!isOpen) return null;

  const handleChange = (key: keyof SimulationParams, value: any) => {
    onUpdate({ ...params, [key]: value });
  };

  return (
    <div className="absolute inset-0 z-[110] bg-slate-900/40 backdrop-blur-sm flex items-center justify-center p-4 font-sans">
      <div className="bg-white w-full max-w-md rounded-2xl shadow-2xl border border-slate-200 overflow-hidden">
        {/* Header */}
        <div className="p-4 border-b border-slate-100 flex items-center justify-between bg-slate-50/50">
          <div className="flex items-center gap-2">
            <Settings size={18} className="text-slate-400" />
            <h2 className="font-bold text-slate-800 uppercase tracking-tight text-sm">Ajustes de Simulación</h2>
          </div>
          <button onClick={onClose} className="text-slate-400 hover:text-slate-600 transition-colors">
            <X size={20} />
          </button>
        </div>

        {/* Body */}
        <div className="p-6 space-y-6">
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-1.5">
              <label className="text-[10px] font-black text-slate-400 uppercase tracking-widest">Tiempo Final (s)</label>
              <input 
                type="number" 
                value={params.t_final} 
                onChange={(e) => handleChange('t_final', parseFloat(e.target.value))}
                className="w-full p-2.5 bg-slate-50 border border-slate-200 rounded-lg text-sm font-mono focus:ring-2 focus:ring-blue-500 outline-none"
              />
            </div>
            <div className="space-y-1.5">
              <label className="text-[10px] font-black text-slate-400 uppercase tracking-widest">Paso (dt)</label>
              <input 
                type="number" 
                value={params.dt} 
                onChange={(e) => handleChange('dt', parseFloat(e.target.value))}
                className="w-full p-2.5 bg-slate-50 border border-slate-200 rounded-lg text-sm font-mono focus:ring-2 focus:ring-blue-500 outline-none"
              />
            </div>
          </div>

          <div className="space-y-1.5">
            <label className="text-[10px] font-black text-slate-400 uppercase tracking-widest">Tipo de Solver</label>
            <select 
              value={params.solver} 
              onChange={(e) => handleChange('solver', e.target.value)}
              className="w-full p-2.5 bg-slate-50 border border-slate-200 rounded-lg text-sm font-bold text-slate-700 focus:ring-2 focus:ring-blue-500 outline-none appearance-none"
            >
              <option value="Euler">Euler (Paso Fijo)</option>
              <option value="RK4">Runge-Kutta 4 (Paso Fijo)</option>
              <option value="RK45">Dormand-Prince (Paso Variable)</option>
            </select>
          </div>

          {params.solver === 'RK45' && (
            <div className="grid grid-cols-2 gap-4 pt-2 border-t border-slate-100 mt-4">
              <div className="space-y-1.5">
                <label className="text-[10px] font-black text-slate-400 uppercase tracking-widest text-blue-600">Abs Tol</label>
                <input 
                  type="number" 
                  value={params.atol} 
                  onChange={(e) => handleChange('atol', parseFloat(e.target.value))}
                  className="w-full p-2.5 bg-blue-50/30 border border-blue-100 rounded-lg text-sm font-mono focus:ring-2 focus:ring-blue-500 outline-none"
                />
              </div>
              <div className="space-y-1.5">
                <label className="text-[10px] font-black text-slate-400 uppercase tracking-widest text-blue-600">Rel Tol</label>
                <input 
                  type="number" 
                  value={params.rtol} 
                  onChange={(e) => handleChange('rtol', parseFloat(e.target.value))}
                  className="w-full p-2.5 bg-blue-50/30 border border-blue-100 rounded-lg text-sm font-mono focus:ring-2 focus:ring-blue-500 outline-none"
                />
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-4 bg-slate-50 border-t border-slate-100 flex justify-end">
          <button 
            onClick={onClose}
            className="px-6 py-2 bg-slate-900 text-white rounded-lg font-bold text-xs hover:bg-slate-800 transition-all uppercase tracking-widest shadow-lg"
          >
            Aplicar
          </button>
        </div>
      </div>
    </div>
  );
};
