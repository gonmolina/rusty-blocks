import React from 'react';
import { 
  LineChart, 
  Line, 
  XAxis, 
  YAxis, 
  CartesianGrid, 
  Tooltip, 
  Legend, 
  ResponsiveContainer 
} from 'recharts';

interface SimulationPoint {
  t: number;
  x: number[];
}

interface ResultsChartProps {
  data: SimulationPoint[];
}

export const ResultsChart: React.FC<ResultsChartProps> = ({ data }) => {
  if (!data || data.length === 0 || !data[0].x) return null;

  // Reformat data for Recharts: { t, x0, x1, ... }
  const chartData = data.map(point => {
    const entry: any = { t: parseFloat(point.t.toFixed(4)) };
    if (point.x) {
      point.x.forEach((val, i) => {
        entry[`state_${i}`] = val;
      });
    }
    return entry;
  });

  const numStates = data[0].x?.length || 0;
  if (numStates === 0) {
    return (
      <div className="bg-white p-6 rounded-xl border border-slate-200 shadow-xl h-full flex flex-col items-center justify-center text-slate-400">
        <p className="text-xs font-bold uppercase tracking-widest">Simulación completada (Sin estados continuos)</p>
        <p className="text-[10px]">Usa un Scope para ver señales algebraicas.</p>
      </div>
    );
  }

  const colors = ['#3b82f6', '#ef4444', '#10b981', '#f59e0b', '#8b5cf6', '#ec4899'];

  return (
    <div className="bg-white p-6 rounded-xl border border-slate-200 shadow-xl h-full flex flex-col">
      <h3 className="text-sm font-bold text-slate-700 uppercase tracking-wider mb-4 flex items-center gap-2">
        <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse" />
        Resultados de la Simulación
      </h3>
      <div className="flex-1 min-h-0">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={chartData}>
            <CartesianGrid strokeDasharray="3 3" stroke="#f1f5f9" />
            <XAxis 
              dataKey="t" 
              label={{ value: 'Tiempo (s)', position: 'insideBottomRight', offset: -5 }} 
              fontSize={10}
            />
            <YAxis fontSize={10} />
            <Tooltip 
              contentStyle={{ backgroundColor: '#fff', borderRadius: '8px', border: '1px solid #e2e8f0', fontSize: '12px' }}
            />
            <Legend verticalAlign="top" height={36}/>
            {Array.from({ length: numStates }).map((_, i) => (
              <Line
                key={i}
                type="monotone"
                dataKey={`state_${i}`}
                name={`Estado ${i}`}
                stroke={colors[i % colors.length]}
                strokeWidth={2}
                dot={false}
                animationDuration={300}
              />
            ))}
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
};
