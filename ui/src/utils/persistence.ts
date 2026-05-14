import type { Node, Edge } from 'reactflow';

export interface ProjectFile {
  version: string;
  system: {
    name: string;
    blocks: any[];
    connections: any[];
  };
  ui: {
    nodes: Node[];
    edges: Edge[];
  };
}

export const exportProject = (name: string, nodes: Node[], edges: Edge[]): ProjectFile => {
  const blocks = nodes.map(node => ({
    id: node.id,
    type: node.type,
    params: node.data.params,
    // Guardamos la posición aquí también por si el motor quiere usarla
    ui_meta: { position: node.position } 
  }));

  const connections = edges.map(edge => ({
    from: edge.source,
    from_port: parseInt(edge.sourceHandle?.split('-')[1] || '0'),
    to: edge.target,
    to_port: parseInt(edge.targetHandle?.split('-')[1] || '0'),
  }));

  return {
    version: "2.0",
    system: {
      name,
      blocks,
      connections
    },
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
