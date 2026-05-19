import React, { useEffect, useState, useRef } from 'react';
import { X, LayoutDashboard, Printer } from 'lucide-react';
import {
  ResponsiveContainer,
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
} from 'recharts';
import type { Node, Edge } from 'reactflow';

interface SimulationPoint {
  t: number;
  x: number[];
  y: number[];
}

interface DashboardProps {
  isOpen: boolean;
  onClose: () => void;
  results: SimulationPoint[];
  yOffsets: Record<string, number>;
  outputWidths: Record<string, number[]>;
  nodes: Node[];
  edges: Edge[];
}

const extractScopeData = (
  nodeId: string,
  results: SimulationPoint[],
  yOffsets: Record<string, number>,
  outputWidths: Record<string, number[]>,
  edges: Edge[]
): any[] => {
  if (results.length === 0) return [];
  const incomingEdges = edges.filter(e => e.target === nodeId);
  if (incomingEdges.length === 0) return [];
  return results.map(p => {
    const entry: any = { t: p.t };
    let chIdx = 1;
    incomingEdges.forEach((edge) => {
      const sourceBlockId = edge.source;
      const sourcePortIdx = parseInt(edge.sourceHandle?.split('-')[1] || '0');
      const blockOffset = yOffsets[sourceBlockId];
      if (blockOffset === undefined) return;
      const portWidths = outputWidths[sourceBlockId] || [1];
      let portByteOffset = 0;
      for (let p = 0; p < sourcePortIdx; p++) {
        portByteOffset += portWidths[p] || 0;
      }
      const width = portWidths[sourcePortIdx] || 1;
      const startIdx = blockOffset + portByteOffset;
      for (let w = 0; w < width; w++) {
        entry[`ch${chIdx}`] = p.y[startIdx + w];
        chIdx++;
      }
    });
    return entry;
  });
};

export const Dashboard: React.FC<DashboardProps> = ({ isOpen, onClose, results, yOffsets, outputWidths, nodes, edges }) => {
  const [fileSinkData, setFileSinkData] = useState<Record<string, any[]>>({});
  const dashboardRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!isOpen) return;
    const fileSinkNodes = nodes.filter(n => n.type === 'FileSink');
    const fetchPromises = fileSinkNodes.map(async (node) => {
      const filename = node.data.params?.filename;
      if (!filename) return { id: node.id, data: [] };
      try {
        const apiHost = window.location.hostname;
        const response = await fetch(`http://${apiHost}:3000/results/${filename}`);
        if (response.ok) {
          const data = await response.json();
          return { id: node.id, data };
        }
      } catch { /* file not found — skip */ }
      return { id: node.id, data: [] };
    });
    Promise.all(fetchPromises).then(results => {
      const map: Record<string, any[]> = {};
      results.forEach(r => { if (r) map[r.id] = r.data; });
      setFileSinkData(map);
    });
  }, [isOpen, nodes]);

  const handlePrint = () => window.print();

  if (!isOpen) return null;

  const defaultColors = ['#2563eb', '#16a34a', '#dc2626', '#ca8a04'];
  const cards: { id: string; title: string; data: any[]; colors: string[]; loading?: boolean }[] = [];

  nodes.forEach(node => {
    if (node.type === 'Scope') {
      const data = extractScopeData(node.id, results, yOffsets, outputWidths, edges);
      if (data.length > 0) {
        cards.push({
          id: node.id,
          title: node.data.params?.name || 'Scope',
          data,
          colors: node.data.params?.viz_config?.colors || defaultColors,
        });
      }
    } else if (node.type === 'FileSink') {
      const data = fileSinkData[node.id];
      cards.push({
        id: node.id,
        title: node.data.params?.filename || 'FileSink',
        data: data || [],
        colors: defaultColors,
        loading: !data,
      });
    }
  });

  return (
    <div ref={dashboardRef} className="fixed inset-0 z-[150] flex flex-col bg-slate-100 overflow-hidden print:relative print:z-auto print:h-auto print:overflow-visible">
      {/* Toolbar */}
      <div className="flex items-center justify-between px-8 py-4 bg-white border-b border-slate-200 shadow-sm print:hidden z-10">
        <div className="flex items-center gap-4">
          <div className="w-10 h-10 bg-slate-900 rounded-xl flex items-center justify-center text-white shadow-lg">
            <LayoutDashboard size={20} />
          </div>
          <div>
            <h2 className="text-xl font-black text-slate-800 uppercase tracking-tight">Signal Monitoring Center</h2>
            <p className="text-[10px] text-slate-400 font-bold uppercase tracking-widest">
              {cards.length} gr\u00e1fico{cards.length !== 1 ? 's' : ''} sincronizado{cards.length !== 1 ? 's' : ''}
            </p>
          </div>
        </div>
        <div className="flex items-center gap-3">
          <button onClick={handlePrint} className="flex items-center gap-2 px-4 py-2.5 bg-white border border-slate-200 rounded-xl text-xs font-bold text-slate-600 hover:bg-slate-50 hover:text-blue-600 transition-all shadow-sm">
            <Printer size={14} /> Exportar PDF
          </button>
          <button onClick={onClose} className="p-2.5 bg-slate-100 text-slate-400 rounded-xl hover:bg-red-50 hover:text-red-500 transition-all">
            <X size={24} />
          </button>
        </div>
      </div>

      {/* Grid */}
      <div className="flex-1 overflow-y-auto p-6">
        {cards.length === 0 ? (
          <div className="h-full flex flex-col items-center justify-center text-slate-300 gap-4">
            <div className="w-20 h-20 border-4 border-slate-100 border-dashed rounded-full animate-spin duration-[3s]" />
            <p className="text-sm font-bold uppercase tracking-widest">No hay Scopes ni FileSinks en este sistema</p>
            <p className="text-[10px] text-slate-400">Agreg\u00e1 bloques Scope o FileSink al canvas, simul\u00e1, y volv\u00e9 a abrir el Dashboard.</p>
          </div>
        ) : (
          <div className="grid grid-cols-1 xl:grid-cols-2 gap-6 print:grid-cols-2">
            {cards.map(card => (
              <div key={card.id} className="bg-white rounded-2xl border border-slate-200 shadow-lg overflow-hidden flex flex-col print:border-slate-300" style={{ minHeight: '320px' }}>
                <div className="px-6 py-4 bg-slate-50/80 border-b border-slate-100 flex items-center gap-3">
                  <div className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
                  <h3 className="text-sm font-black text-slate-700 uppercase tracking-wider">{card.title}</h3>
                  {card.loading && <span className="text-[9px] font-bold text-amber-500 uppercase ml-auto">Cargando...</span>}
                </div>
                <div className="flex-1 p-4 min-h-0">
                  {card.loading ? (
                    <div className="h-full flex items-center justify-center">
                      <div className="w-8 h-8 border-3 border-slate-200 border-t-blue-500 rounded-full animate-spin" />
                    </div>
                  ) : card.data.length === 0 ? (
                    <div className="h-full flex items-center justify-center text-slate-300 text-xs font-bold uppercase">Sin datos — ejecut\u00e1 la simulaci\u00f3n</div>
                  ) : (
                    <ResponsiveContainer width="100%" height="100%">
                      <LineChart data={card.data} syncId="dashboard-sync">
                        <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="#f1f5f9" />
                        <XAxis dataKey="t" type="number" domain={['auto', 'auto']} stroke="#94a3b8" fontSize={10} tickFormatter={(val) => `${val.toFixed(1)}s`} />
                        <YAxis stroke="#94a3b8" fontSize={10} width={50} />
                        <Tooltip cursor={{ stroke: '#94a3b8', strokeWidth: 1, strokeDasharray: '4 4' }} contentStyle={{ backgroundColor: 'rgba(255,255,255,0.95)', borderRadius: '10px', border: '1px solid #e2e8f0', fontSize: '11px' }} />
                        <Legend iconType="circle" wrapperStyle={{ paddingTop: '8px', fontSize: '10px' }} />
                        {Object.keys(card.data[0]).filter(k => k !== 't').map((key, i) => (
                          <Line key={key} type="monotone" dataKey={key} stroke={card.colors[i] || defaultColors[i % 4]} strokeWidth={2} dot={false} animationDuration={300} />
                        ))}
                      </LineChart>
                    </ResponsiveContainer>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
