import React from 'react';
import { X, Maximize2, Download } from 'lucide-react';
import { 
  ResponsiveContainer, 
  LineChart, 
  Line, 
  XAxis, 
  YAxis, 
  CartesianGrid, 
  Tooltip, 
  Legend,
  Brush
} from 'recharts';

interface ScopeModalProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  data: any[];
  vizConfig?: {
    colors?: string[];
    y_min?: number;
    y_max?: number;
    t_min?: number;
    t_max?: number;
  };
}

export const ScopeModal: React.FC<ScopeModalProps> = ({ isOpen, onClose, title, data, vizConfig }) => {
  if (!isOpen) return null;

  const downloadCSV = () => {
    if (data.length === 0) return;
    const headers = Object.keys(data[0]).join(',');
    const rows = data.map(point => Object.values(point).join(','));
    const csvContent = [headers, ...rows].join('\n');
    const blob = new Blob([csvContent], { type: 'text/csv' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `${title.replace(/\s+/g, '_')}_data.csv`;
    link.click();
  };

  return (
    <div className="fixed inset-0 z-[200] flex items-center justify-center p-4">
      <div className="absolute inset-0 bg-slate-900/60 backdrop-blur-sm" onClick={onClose} />
      
      <div className="relative bg-white w-full max-w-5xl h-[80vh] rounded-3xl shadow-2xl border border-slate-200 flex flex-col overflow-hidden animate-in fade-in zoom-in duration-200">
        {/* Header */}
        <div className="flex items-center justify-between px-8 py-6 border-b border-slate-100 bg-slate-50/50">
          <div className="flex items-center gap-4">
            <div className="w-10 h-10 bg-green-600 rounded-xl flex items-center justify-center text-white shadow-lg">
              <Maximize2 size={20} />
            </div>
            <div>
              <h2 className="text-xl font-black text-slate-800 uppercase tracking-tight">{title}</h2>
              <p className="text-[10px] text-slate-400 font-bold uppercase tracking-widest">Visualización de Señal en Tiempo Real</p>
            </div>
          </div>
          
          <div className="flex items-center gap-2">
            <button 
              onClick={downloadCSV}
              className="flex items-center gap-2 px-4 py-2 bg-white border border-slate-200 rounded-xl text-xs font-bold text-slate-600 hover:bg-slate-50 hover:text-blue-600 transition-all shadow-sm"
            >
              <Download size={14} /> Exportar CSV
            </button>
            <button 
              onClick={onClose}
              className="p-2 bg-slate-100 text-slate-400 rounded-xl hover:bg-red-50 hover:text-red-500 transition-all"
            >
              <X size={24} />
            </button>
          </div>
        </div>

        {/* Content */}
        <div className="flex-1 p-8 min-h-0 bg-white relative">
          {data.length > 0 ? (
            <ResponsiveContainer width="100%" height="100%">
              <LineChart data={data}>
                <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="#f1f5f9" />
                <XAxis 
                  dataKey="t" 
                  type="number" 
                  domain={[
                    vizConfig?.t_min ?? 'auto',
                    vizConfig?.t_max ?? 'auto'
                  ]} 
                  stroke="#94a3b8" 
                  fontSize={10} 
                  tickFormatter={(val) => `${val.toFixed(2)}s`}
                />
                <YAxis 
                  stroke="#94a3b8" 
                  fontSize={10} 
                  domain={[
                    vizConfig?.y_min ?? 'auto',
                    vizConfig?.y_max ?? 'auto'
                  ]}
                />
                <Tooltip 
                  contentStyle={{ backgroundColor: 'rgba(255, 255, 255, 0.95)', borderRadius: '12px', border: '1px solid #e2e8f0', boxShadow: '0 10px 15px -3px rgba(0, 0, 0, 0.1)' }}
                  labelStyle={{ fontWeight: 'bold', color: '#1e293b' }}
                />
                <Legend iconType="circle" wrapperStyle={{ paddingTop: '20px' }} />
                {Object.keys(data[0]).filter(k => k !== 't').map((key, i) => {
                  const defaultColors = ['#2563eb', '#16a34a', '#dc2626', '#ca8a04'];
                  const customColors = vizConfig?.colors || defaultColors;
                  return (
                   <Line 
                    key={key} 
                    type="monotone" 
                    dataKey={key} 
                    stroke={customColors[i] || defaultColors[i % 4]} 
                    strokeWidth={3} 
                    dot={false}
                    animationDuration={500}
                  />
                  );
                })}
                <Brush dataKey="t" height={30} stroke="#94a3b8" fill="#f8fafc" />
              </LineChart>
            </ResponsiveContainer>
          ) : (
            <div className="w-full h-full flex flex-col items-center justify-center text-slate-300 gap-4">
              <div className="w-20 h-20 border-4 border-slate-100 border-dashed rounded-full animate-spin duration-[3s]" />
              <p className="text-sm font-bold uppercase tracking-widest">Sin datos de simulación disponibles</p>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="px-8 py-4 bg-slate-50 border-t border-slate-100 flex justify-between items-center text-[10px] text-slate-400 font-bold uppercase tracking-widest">
          <span>Resolución: Adaptativa</span>
          <span>Motor de Renderizado: Canvas/SVG</span>
        </div>
      </div>
    </div>
  );
};
