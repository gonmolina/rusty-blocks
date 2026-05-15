import { useCallback, useMemo, useState, useRef, useEffect } from 'react';
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
} from 'reactflow';
import 'reactflow/dist/style.css';

import { 
  IntegratorNode, 
  GainNode, 
  ConstantNode, 
  SumNode, 
  StepNode, 
  MuxNode, 
  DemuxNode, 
  InPortNode, 
  OutPortNode, 
  FileSinkNode, 
  SubsystemNode,
  ScopeNode
} from './components/nodes/BlockNodes';
import { PropertiesPanel } from './components/PropertiesPanel';
import { Sidebar } from './components/Sidebar';
import { ResultsChart } from './components/ResultsChart';
import { SimulationSettings, type SimulationParams } from './components/SimulationSettings';
import { ScopeModal } from './components/ScopeModal';
import { exportProject, downloadJson, handleLoad, convertToSystemConfig } from './utils/persistence';

interface ViewLevel {
  id: string;
  name: string;
  nodes: Node[];
  edges: Edge[];
}

let id_counter = 0;
const getId = () => `node_${Date.now()}_${id_counter++}`;

const FlowEditor = () => {
  const reactFlowWrapper = useRef<HTMLDivElement>(null);
  
  const [viewStack, setViewStack] = useState<ViewLevel[]>([
    { id: 'root', name: 'Main', nodes: [
        { id: 'c1', type: 'Constant', position: { x: 50, y: 150 }, data: { params: { value: [1.0] }, rotation: 0 } },
        { id: 'int1', type: 'Integrator', position: { x: 300, y: 150 }, data: { params: { ic: [0.0] }, rotation: 0 } }
      ], 
      edges: [{ id: 'e1', source: 'c1', target: 'int1', targetHandle: 'in-0' }] 
    }
  ]);
  
  const currentLevel = viewStack[viewStack.length - 1];

  const [nodes, setNodes, onNodesChange] = useNodesState(currentLevel.nodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(currentLevel.edges);
  const [selectedNode, setSelectedNode] = useState<Node | null>(null);
  const [results, setResults] = useState<any[]>([]);
  const [yOffsets, setYOffsets] = useState<Record<string, number>>({});
  const [isSimulating, setIsSimulating] = useState(false);
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [scopeModalState, setScopeModalState] = useState<{ isOpen: boolean; title: string; data: any[] }>({ isOpen: false, title: '', data: [] });
  const [simParams, setSimParams] = useState<SimulationParams>({
    dt: 0.1, t_final: 10.0, solver: 'RK45', atol: 1e-8, rtol: 1e-4
  });
  
  const { screenToFlowPosition } = useReactFlow();

  useEffect(() => {
    setNodes(currentLevel.nodes);
    setEdges(currentLevel.edges);
  }, [currentLevel.id, setNodes, setEdges]);

  useEffect(() => {
    setViewStack(prev => {
      const next = [...prev];
      const currentIndex = next.length - 1;
      if (currentIndex < 0) return prev;
      if (next[currentIndex].nodes === nodes && next[currentIndex].edges === edges) return prev;
      next[currentIndex] = { ...next[currentIndex], nodes, edges };
      let i = currentIndex;
      while (i > 0) {
        const parentIdx = i - 1;
        const currentId = next[i].id;
        const currentNodes = next[i].nodes;
        const currentEdges = next[i].edges;
        next[parentIdx].nodes = next[parentIdx].nodes.map(n => {
          if (n.id === currentId) {
            return { ...n, data: { ...n.data, internalState: { nodes: currentNodes, edges: currentEdges } } };
          }
          return n;
        });
        i--;
      }
      return next;
    });
  }, [nodes, edges]);

  const nodeTypes = useMemo(() => ({
    Constant: ConstantNode,
    Integrator: IntegratorNode,
    Gain: GainNode,
    Sum: SumNode,
    Step: StepNode,
    Mux: MuxNode,
    Demux: DemuxNode,
    InPort: InPortNode,
    OutPort: OutPortNode,
    FileSink: FileSinkNode,
    Subsystem: SubsystemNode,
    Scope: ScopeNode,
  }), []);

  const onConnect = useCallback(
    (params: Connection) => setEdges((eds) => addEdge(params, eds)),
    [setEdges]
  );

  const onNodeDoubleClick = useCallback(async (_event: React.MouseEvent, node: Node) => {
    if (node.type === 'Subsystem') {
      setViewStack(prev => {
        const next = [...prev];
        next[next.length - 1] = { ...currentLevel, nodes, edges };
        const internalState = node.data.internalState || { nodes: [], edges: [] };
        next.push({
          id: node.id,
          name: node.data.params?.name || 'Subsystem',
          nodes: internalState.nodes,
          edges: internalState.edges
        });
        return next;
      });
    } else if (node.type === 'Scope') {
      setScopeModalState({ 
        isOpen: true, 
        title: node.data.params?.name || "Scope Viewer", 
        data: getScopeData(node.id) 
      });
    } else if (node.type === 'FileSink') {
      const filename = node.data.params?.filename || 'output.csv';
      const apiHost = window.location.hostname;
      try {
        const response = await fetch(`http://${apiHost}:3000/results/${filename}`);
        if (!response.ok) throw new Error("File not found");
        const data = await response.json();
        setScopeModalState({ 
          isOpen: true, 
          title: `File: ${filename}`, 
          data: data 
        });
      } catch (err) {
        alert(`No se pudo recuperar el archivo ${filename}. ¿Ya ejecutaste la simulación?`);
      }
    }
  }, [nodes, edges, currentLevel, results, yOffsets]);

  const navigateBack = (index: number) => {
    if (index === viewStack.length - 1) return;
    setViewStack(prev => {
      const next = [...prev];
      const currentSubId = prev[prev.length - 1].id;
      next[index].nodes = next[index].nodes.map(n => {
        if (n.id === currentSubId) {
          return { 
            ...n, 
            data: { 
              ...n.data, 
              internalState: { nodes, edges },
              params: {
                ...n.data.params,
                _numInputs: nodes.filter(node => node.type === 'InPort').length,
                _numOutputs: nodes.filter(node => node.type === 'OutPort').length
              }
            } 
          };
        }
        return n;
      });
      return next.slice(0, index + 1);
    });
  };

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
        data: { params: getDefaultParams(type), rotation: 0 },
      };
      setNodes((nds) => nds.concat(newNode));
    },
    [screenToFlowPosition, setNodes]
  );

  const handleSimulate = async () => {
    setIsSimulating(true);
    const rootLevel = viewStack[0];
    const system = convertToSystemConfig("Full System", rootLevel.nodes, rootLevel.edges);
    const apiHost = window.location.hostname;
    const apiUrl = `http://${apiHost}:3000/simulate`;

    try {
      const response = await fetch(apiUrl, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ system, params: simParams }),
      });
      const data = await response.json();
      setResults(data.points);
      setYOffsets(data.y_offsets);
    } catch (err) {
      alert("Error al conectar con el servidor de simulación");
    } finally {
      setIsSimulating(false);
    }
  };

  const getScopeData = (nodeId: string) => {
    if (results.length === 0) return [];
    const incomingEdge = edges.find(e => e.target === nodeId);
    if (!incomingEdge) return [];
    const sourceBlockId = incomingEdge.source;
    const sourcePort = parseInt(incomingEdge.sourceHandle?.split('-')[1] || '0');
    const offset = yOffsets[sourceBlockId];
    if (offset === undefined) return [];
    return results.map(p => ({
      t: p.t,
      signal: p.y[offset + sourcePort]
    }));
  };

  return (
    <div className="flex w-full h-full relative overflow-hidden bg-slate-100 font-sans" ref={reactFlowWrapper}>
      <Sidebar 
        onSave={() => downloadJson(exportProject("Project", nodes, edges), "project.json")} 
        onLoad={(file) => handleLoad(file, setNodes, setEdges)} 
        onSimulate={handleSimulate} 
        onOpenSettings={() => setIsSettingsOpen(true)}
        solverType={simParams.solver}
      />
      
      <div className="flex-1 h-full flex flex-col">
        <div className="bg-white/80 backdrop-blur-md px-6 py-3 border-b border-slate-200 flex items-center gap-2 z-20">
          {viewStack.map((level, i) => (
            <div key={level.id} className="flex items-center gap-2">
              {i > 0 && <span className="text-slate-300 font-bold">/</span>}
              <button 
                onClick={() => navigateBack(i)}
                className={`text-xs font-black uppercase tracking-widest hover:text-blue-600 transition-colors ${i === viewStack.length - 1 ? 'text-slate-800' : 'text-slate-400'}`}
              >
                {level.name}
              </button>
            </div>
          ))}
        </div>

        <div className={results.length > 0 ? "h-2/3 border-b border-slate-200" : "flex-1"}>
          <ReactFlow
            nodes={nodes}
            edges={edges}
            nodeTypes={nodeTypes}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            onConnect={onConnect}
            onDrop={onDrop}
            onDragOver={(e) => { e.preventDefault(); e.dataTransfer.dropEffect = 'move'; }}
            onSelectionChange={(p) => setSelectedNode(p.nodes[0] || null)}
            onNodeDoubleClick={onNodeDoubleClick}
            fitView
          >
            <Background />
            <Controls />
            <MiniMap />
          </ReactFlow>
        </div>

        {results.length > 0 && (
          <div className="h-1/3 bg-slate-50 p-4 relative overflow-hidden">
             <button onClick={() => setResults([])} className="absolute top-6 right-6 z-20 text-xs font-bold text-slate-400 hover:text-slate-600 uppercase transition-colors">Cerrar</button>
            <ResultsChart data={results} />
          </div>
        )}
      </div>

      <SimulationSettings isOpen={isSettingsOpen} onClose={() => setIsSettingsOpen(false)} params={simParams} onUpdate={setSimParams} />
      
      <ScopeModal 
        isOpen={scopeModalState.isOpen} 
        onClose={() => setScopeModalState({ ...scopeModalState, isOpen: false })}
        title={scopeModalState.title}
        data={scopeModalState.data}
      />

      {isSimulating && (
        <div className="absolute inset-0 z-[100] bg-slate-900/20 backdrop-blur-[2px] flex items-center justify-center">
          <div className="bg-white p-8 rounded-3xl shadow-2xl flex flex-col items-center gap-4 border border-slate-200">
            <div className="w-12 h-12 border-4 border-blue-600 border-t-transparent rounded-full animate-spin"></div>
            <span className="text-sm font-black text-slate-800 uppercase tracking-widest text-center">Ejecutando Simulación...</span>
          </div>
        </div>
      )}

      <PropertiesPanel 
        selectedNode={nodes.find(n => n.id === selectedNode?.id) || null} 
        onUpdateParams={(id, p) => setNodes(nds => nds.map(n => n.id === id ? { ...n, data: { ...n.data, params: p } } : n))}
        onUpdateRotation={(id, r) => setNodes(nds => nds.map(n => n.id === id ? { ...n, data: { ...n.data, rotation: r } } : n))}
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
    case 'Scope': return { name: "Scope" };
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
