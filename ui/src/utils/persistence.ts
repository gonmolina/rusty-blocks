import type { Node, Edge } from 'reactflow';

export interface ProjectFile {
  version: string;
  system: SystemConfig;
  ui: {
    nodes: Node[];
    edges: Edge[];
  };
}

interface SystemConfig {
  name: string;
  blocks: any[];
  connections: any[];
}

export const convertToSystemConfig = (name: string, nodes: Node[], edges: Edge[]): SystemConfig => {
  const blocks = nodes.map(node => {
    let params = { ...node.data.params };

    // Si es un subsistema, construimos su configuración interna recursivamente
    if (node.type === 'Subsystem' && node.data.internalState) {
      const internal = node.data.internalState;
      const subConfig = convertToSystemConfig(
        params.name || "Subsystem",
        internal.nodes || [],
        internal.edges || []
      );
      // El motor espera que los parámetros de un Subsystem sean el SystemConfig mismo
      params = { ...subConfig };
    }

    return {
      id: node.id,
      type: node.type,
      params: params,
      ui_meta: { position: node.position, rotation: node.data.rotation }
    };
  });

  const connections = edges.map(edge => ({
    from: edge.source,
    from_port: parseInt(edge.sourceHandle?.split('-')[1] || '0'),
    to: edge.target,
    to_port: parseInt(edge.targetHandle?.split('-')[1] || '0'),
  }));

  return {
    name,
    blocks,
    connections
  };
};

export const exportProject = (name: string, nodes: Node[], edges: Edge[]): ProjectFile => {
  return {
    version: "2.0",
    system: convertToSystemConfig(name, nodes, edges),
    ui: {
      nodes,
      edges
    }
  };
};

export const downloadJson = (data: object, filename: string) => {
  const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
};

export const handleLoad = (file: File, setNodes: any, setEdges: any) => {
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
};
