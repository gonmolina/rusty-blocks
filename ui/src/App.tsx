import { useCallback, useMemo, useState, useRef } from 'react';
import ReactFlow, { 
  addEdge, 
  Background, 
  Controls, 
  MiniMap, 
  useNodesState, 
  useEdgesState,
  ReactFlowProvider,
  useReactFlow
} from 'reactflow';
import type {
  Connection,
  Edge,
  Node,
  OnSelectionChangeParams
} from 'reactflow';
import 'reactflow/dist/style.css';

import { IntegratorNode, GainNode, ConstantNode, SumNode } from './components/nodes/BlockNodes';
import { PropertiesPanel } from './components/PropertiesPanel';
import { Sidebar } from './components/Sidebar';
import { ResultsChart } from './components/ResultsChart';
import { SimulationSettings, type SimulationParams } from './components/SimulationSettings';
import { exportProject, downloadJson, type ProjectFile } from './utils/persistence';

const initialNodes: Node[] = [
  { 
    id: 'c1', 
    type: 'Constant',
    position: { x: 50, y: 150 }, 
    data: { params: { value: [1.0] } }
  },
  { 
    id: 'int1', 
    type: 'Integrator',
    position: { x: 300, y: 150 }, 
    data: { params: { ic: [0.0] } }
  }
];

const initialEdges: Edge[] = [
  { id: 'e1', source: 'c1', target: 'int1', targetHandle: 'in-0' }
];

let id_counter = 0;
const getId = () => `node_${Date.now()}_${id_counter++}`;

const FlowEditor = () => {
  const reactFlowWrapper = useRef<HTMLDivElement>(null);
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);
  const [selectedNode, setSelectedNode] = useState<Node | null>(null);
  const [results, setResults] = useState<any[]>([]);
  const [isSimulating, setIsSimulating] = useState(false);
  
  // Simulation Settings State
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [simParams, setSimParams] = useState<SimulationParams>({
    dt: 0.1,
    t_final: 10.0,
    solver: 'RK45',
    atol: 1e-8,
    rtol: 1e-4
  });
  
  const { screenToFlowPosition } = useReactFlow();

  const nodeTypes = useMemo(() => ({
    Constant: ConstantNode,
    Integrator: IntegratorNode,
    Gain: GainNode,
    Sum: SumNode,
  }), []);

  const onConnect = useCallback(
    (params: Connection) => setEdges((eds) => addEdge(params, eds)),
    [setEdges]
  );

  const onDragOver = useCallback((event: React.DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
  }, []);

  const onDrop = useCallback(
    (event: React.DragEvent) => {
      event.preventDefault();
      const type = event.dataTransfer.getData('application/reactflow');
      if (!type) return;
      const position = screenToFlowPosition({ x: event.clientX, y: event.clientY });
      const newNode: Node = {
        id: getId(),
        type,
        position,
        data: { params: getDefaultParams(type) },
      };
      setNodes((nds) => nds.concat(newNode));
    },
    [screenToFlowPosition, setNodes]
  );

  const handleSave = useCallback(() => {
    const project = exportProject("Mi Simulación", nodes, edges);
    downloadJson(project, "modelo_bloques.json");
  }, [nodes, edges]);

  const handleLoad = useCallback((file: File) => {
    const reader = new FileReader();
    reader.onload = (e) => {
      try {
        const project: ProjectFile = JSON.parse(e.target?.result as string);
        if (project.ui) {
          setNodes(project.ui.nodes || []);
          setEdges(project.ui.edges || []);
        }
      } catch (err) {
        alert("Error al cargar el archivo JSON");
      }
    };
    reader.readAsText(file);
  }, [setNodes, setEdges]);

  const handleSimulate = async () => {
    setIsSimulating(true);
    const system = exportProject("Simulación Web", nodes, edges).system;

    try {
      const response = await fetch('http://localhost:3000/simulate', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ system, params: simParams }),
      });
      
      const data = await response.json();
      setResults(data.points);
    } catch (err) {
      alert("Error al conectar con el servidor de simulación");
    } finally {
      setIsSimulating(false);
    }
  };

  const updateNodeParams = useCallback((nodeId: string, newParams: any) => {
    setNodes((nds) =>
      nds.map((node) => {
        if (node.id === nodeId) {
          return { ...node, data: { ...node.data, params: newParams } };
        }
        return node;
      })
    );
  }, [setNodes]);

  const onSelectionChange = useCallback((params: OnSelectionChangeParams) => {
    setSelectedNode(params.nodes[0] || null);
  }, []);

  return (
    <div className="flex w-full h-full relative overflow-hidden bg-slate-100" ref={reactFlowWrapper}>
      <Sidebar 
        onSave={handleSave} 
        onLoad={handleLoad} 
        onSimulate={handleSimulate} 
        onOpenSettings={() => setIsSettingsOpen(true)}
        solverType={simParams.solver}
      />
      
      <div className="flex-1 h-full flex flex-col">
        <div className={results.length > 0 ? "h-2/3 border-b border-slate-200" : "h-full"}>
          <ReactFlow
            nodes={nodes}
            edges={edges}
            nodeTypes={nodeTypes}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            onConnect={onConnect}
            onDrop={onDrop}
            onDragOver={onDragOver}
            onSelectionChange={onSelectionChange}
            fitView
          >
            <Background />
            <Controls />
            <MiniMap />
          </ReactFlow>
        </div>

        {results.length > 0 && (
          <div className="h-1/3 bg-slate-50 p-4 relative">
             <button 
              onClick={() => setResults([])}
              className="absolute top-6 right-6 z-20 text-xs font-bold text-slate-400 hover:text-slate-600 uppercase"
            >
              Cerrar Gráfico
            </button>
            <ResultsChart data={results} />
          </div>
        )}
      </div>

      <SimulationSettings 
        isOpen={isSettingsOpen}
        onClose={() => setIsSettingsOpen(false)}
        params={simParams}
        onUpdate={setSimParams}
      />

      {isSimulating && (
        <div className="absolute inset-0 z-[100] bg-slate-900/20 backdrop-blur-[2px] flex items-center justify-center font-sans">
          <div className="bg-white p-6 rounded-2xl shadow-2xl flex flex-col items-center gap-4 border border-slate-200">
            <div className="w-10 h-10 border-4 border-blue-600 border-t-transparent rounded-full animate-spin"></div>
            <span className="text-sm font-bold text-slate-700 uppercase tracking-widest">Ejecutando Simulación...</span>
          </div>
        </div>
      )}

      <PropertiesPanel 
        selectedNode={nodes.find(n => n.id === selectedNode?.id) || null} 
        onUpdateParams={updateNodeParams} 
      />
    </div>
  );
};

const getDefaultParams = (type: string) => {
  switch (type) {
    case 'Gain': return { k: 1.0, width: 1 };
    case 'Sum': return { signs: '++', width: 1 };
    case 'Integrator': return { ic: [0.0] };
    case 'Constant': return { value: [1.0] };
    case 'Step': return { initial_value: 0.0, final_value: 1.0, step_time: 1.0 };
    case 'FileSink': return { filename: 'output.csv', interval: 0.1, width: 1 };
    case 'Mux': return { input_widths: [1, 1] };
    case 'Demux': return { output_widths: [1, 1] };
    case 'InPort': return { width: 1 };
    case 'OutPort': return { width: 1 };
    case 'Subsystem': return { name: "Subsystem", blocks: [], connections: [] };
    default: return {};
  }
};

export default function App() {
  return (
    <ReactFlowProvider>
      <FlowEditor />
    </ReactFlowProvider>
  );
}
