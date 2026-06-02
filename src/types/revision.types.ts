export interface Revision {
  id: string;
  nodeId: string;
  content: string;
  plainText: string;
  createdAt: number;
}

export interface CreateRevisionInput {
  nodeId: string;
  content: string;
  plainText: string;
}
