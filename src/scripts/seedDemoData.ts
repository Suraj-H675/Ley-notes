/**
 * Demo seed for Knowledge Universe.
 *
 * Loads AI-themed demo content that exercises every feature of the app:
 * - Multiple node types (document, concept, task, project, person)
 * - Wiki-links between pages (using TipTap text marks)
 * - Typed relationships (depends-on, uses, related-to, extends)
 * - Properties (book-note template, person template)
 * - Tags + collections
 * - Revisions (so history page has data)
 *
 * Triggered from the Home page's "Load demo data" button.
 */

import { db } from '@/lib/db';
import {
  createNode,
  createEdge,
  createCollection,
  createRevision,
} from '@/lib/db';
import type { JSONContent } from '@tiptap/react';

interface Block {
  type: 'h1' | 'h2' | 'h3' | 'p' | 'bulletList' | 'orderedList' | 'blockquote' | 'codeBlock';
  text?: string;
  items?: string[];
  language?: string;
}

function doc(blocks: Block[]): JSONContent {
  return {
    type: 'doc',
    content: blocks.map((b) => {
      if (b.type === 'bulletList' || b.type === 'orderedList') {
        return {
          type: b.type,
          content: (b.items || []).map((item) => ({
            type: 'listItem',
            content: [{ type: 'paragraph', content: [{ type: 'text', text: item }] }],
          })),
        };
      }
      if (b.type === 'blockquote') {
        return {
          type: 'blockquote',
          content: [{ type: 'paragraph', content: [{ type: 'text', text: b.text || '' }] }],
        };
      }
      if (b.type === 'codeBlock') {
        return {
          type: 'codeBlock',
          attrs: { language: b.language || null },
          content: [{ type: 'text', text: b.text || '' }],
        };
      }
      const tag = b.type === 'p' ? 'paragraph' : 'heading';
      const attrs =
        b.type === 'h1' ? { level: 1 } : b.type === 'h2' ? { level: 2 } : { level: 3 };
      return {
        type: tag,
        attrs: tag === 'heading' ? attrs : undefined,
        content: [{ type: 'text', text: b.text || '' }],
      };
    }),
  };
}

function extractTextFromTiptap(content: JSONContent): string {
  if (!content) return '';
  const parts: string[] = [];
  function walk(node: JSONContent) {
    if (node.type === 'text' && typeof node.text === 'string') {
      parts.push(node.text);
    }
    if (node.content) {
      node.content.forEach(walk);
      if (['paragraph', 'heading', 'listItem', 'bulletList', 'orderedList', 'codeBlock'].includes(String(node.type))) {
        parts.push('\n');
      }
    }
  }
  walk(content);
  return parts.join('').trim();
}

export async function seedDemoData(): Promise<void> {
  await db.transaction(
    'rw',
    [db.nodes, db.edges, db.collections, db.revisions],
    async () => {
      await db.nodes.clear();
      await db.edges.clear();
      await db.collections.clear();
      await db.revisions.clear();
    }
  );

  // ---------------- Collections ----------------
  const foundations = await createCollection({ name: 'AI Foundations', emoji: '🧠' });
  const applications = await createCollection({ name: 'AI Applications', emoji: '🚀' });
  const peopleCol = await createCollection({ name: 'People', emoji: '👤' });

  // ---------------- Documents ----------------
  const nn = await createNode({
    type: 'document',
    title: 'Neural Networks',
    emoji: '🧠',
    tags: ['foundations', 'ml'],
    collections: [foundations.id],
    template: 'book-note',
    properties: {
      Author: 'Multiple',
      Pages: '120',
      Rating: '5',
      Summary: 'Overview of neural network architectures',
    },
    content: doc([
      { type: 'h1', text: 'Neural Networks' },
      {
        type: 'p',
        text: 'A neural network is a computational model inspired by the human brain. It consists of layers of interconnected neurons that learn patterns from data.',
      },
      { type: 'h2', text: 'Key Components' },
      { type: 'bulletList', items: ['Input layer — receives the raw features', 'Hidden layers — extract and transform features', 'Output layer — produces the prediction'] },
      { type: 'h2', text: 'How Training Works' },
      {
        type: 'p',
        text: 'Training uses backpropagation to compute gradients of a loss function with respect to each weight, then updates the weights via gradient descent.',
      },
      {
        type: 'blockquote',
        text: 'Modern architectures stack many layers to form deep networks. See the Transformers page for the dominant architecture in language tasks.',
      },
    ]),
  });

  const transformers = await createNode({
    type: 'document',
    title: 'Transformers',
    emoji: '⚡',
    tags: ['foundations', 'architecture'],
    collections: [foundations.id],
    template: 'book-note',
    properties: {
      Author: 'Vaswani et al.',
      Pages: '15',
      Rating: '5',
      Summary: 'Attention is all you need',
    },
    content: doc([
      { type: 'h1', text: 'Transformers' },
      {
        type: 'p',
        text: 'The transformer architecture, introduced in "Attention Is All You Need" (2017), replaced recurrence with self-attention. This is the foundation of every modern LLM.',
      },
      { type: 'h2', text: 'Self-Attention' },
      {
        type: 'p',
        text: 'Each token attends to every other token in the sequence, computing a weighted sum based on relevance. This allows modeling long-range dependencies without recurrence.',
      },
      { type: 'h2', text: 'Key Ideas' },
      {
        type: 'bulletList',
        items: [
          'Multi-head attention — run attention in parallel subspaces',
          'Positional encodings — inject order information',
          'Layer normalization — stabilize training',
          'Residual connections — help gradient flow',
        ],
      },
      {
        type: 'p',
        text: 'Used in RAG systems and modern chat assistants. See also Andrej Karpathy for a great walkthrough.',
      },
      {
        type: 'codeBlock',
        language: 'python',
        text: 'import torch\nfrom transformers import AutoModel\n\nmodel = AutoModel.from_pretrained("bert-base-uncased")\noutput = model(input_ids)',
      },
    ]),
  });

  const llm = await createNode({
    type: 'concept',
    title: 'Large Language Models',
    emoji: '🤖',
    tags: ['foundations', 'language'],
    collections: [foundations.id],
    template: 'concept',
    properties: {
      Domain: 'NLP',
      Definition: 'A neural network trained on large text corpora to predict the next token',
      'Related Concepts': 'Transformers, Embeddings, Fine-tuning',
    },
    content: doc([
      { type: 'h1', text: 'Large Language Models' },
      {
        type: 'p',
        text: 'An LLM is a deep neural network (typically a transformer) trained on massive text datasets. The core training objective is next-token prediction.',
      },
      { type: 'h2', text: 'How They Work' },
      {
        type: 'p',
        text: 'At inference time, you prompt the model with a sequence of tokens. It generates one token at a time, sampling from a probability distribution over the vocabulary.',
      },
      { type: 'h2', text: 'Capabilities' },
      {
        type: 'bulletList',
        items: [
          'Text generation, summarization, translation',
          'Code generation and analysis',
          'Reasoning chains (with the right prompting)',
        ],
      },
      {
        type: 'p',
        text: 'Built on Transformers. Trained via Fine-tuning and aligned with RLHF.',
      },
    ]),
  });

  const embeddings = await createNode({
    type: 'concept',
    title: 'Embeddings',
    emoji: '📐',
    tags: ['foundations', 'representation'],
    collections: [foundations.id],
    template: 'concept',
    properties: {
      Domain: 'Representation Learning',
      Definition: 'Dense vector representations of text, images, or other data',
      'Related Concepts': 'RAG, Semantic Search, Cosine Similarity',
    },
    content: doc([
      { type: 'h1', text: 'Embeddings' },
      {
        type: 'p',
        text: 'An embedding is a dense, fixed-size vector that represents a piece of content. Semantically similar inputs map to nearby points in the vector space.',
      },
      { type: 'h2', text: 'Why They Matter' },
      {
        type: 'p',
        text: 'Embeddings enable semantic search: instead of matching keywords, you find content whose meaning is close to the query.',
      },
      {
        type: 'p',
        text: 'They are the backbone of RAG systems, where you embed documents and queries to find relevant context.',
      },
      { type: 'h2', text: 'Common Models' },
      {
        type: 'bulletList',
        items: ['OpenAI text-embedding-3', 'Sentence-Transformers (open source)', 'Cohere embed-v3'],
      },
    ]),
  });

  const rag = await createNode({
    type: 'document',
    title: 'RAG',
    emoji: '🔍',
    tags: ['applications', 'retrieval'],
    collections: [applications.id],
    template: 'research-paper',
    properties: {
      Authors: 'Lewis et al.',
      DOI: '10.48550/arXiv.2005.11401',
      Year: '2020',
      'Key Findings': 'Retrieval-augmented generation reduces hallucination by grounding responses in retrieved documents',
    },
    content: doc([
      { type: 'h1', text: 'Retrieval-Augmented Generation' },
      {
        type: 'p',
        text: 'RAG is a pattern that combines a retriever (typically over Embeddings) with a generator (an LLM). The model is given relevant context before generating a response.',
      },
      { type: 'h2', text: 'Pipeline' },
      {
        type: 'orderedList',
        items: [
          'Embed the user query',
          'Retrieve the top-k most similar chunks from a vector store',
          'Stuff those chunks into the prompt',
          'Let the LLM generate a grounded answer',
        ],
      },
      { type: 'h2', text: 'Tradeoffs' },
      {
        type: 'p',
        text: 'RAG reduces hallucination and lets the model cite sources, but adds latency and depends on retrieval quality. It works best when paired with good chunking and reranking.',
      },
      {
        type: 'p',
        text: 'Used in production chat systems. See the Build internal chatbot task.',
      },
    ]),
  });

  const rlhf = await createNode({
    type: 'document',
    title: 'RLHF',
    emoji: '🎯',
    tags: ['training', 'alignment'],
    collections: [foundations.id],
    template: 'research-paper',
    properties: {
      Authors: 'Christiano et al.',
      DOI: '10.48550/arXiv.1706.03741',
      Year: '2017',
      'Key Findings': 'Reinforcement learning from human feedback aligns models with human preferences',
    },
    content: doc([
      { type: 'h1', text: 'RLHF — Reinforcement Learning from Human Feedback' },
      {
        type: 'p',
        text: 'RLHF is the training technique used to align LLMs with human preferences. It uses human preference data to train a reward model, which then guides the LLM via reinforcement learning.',
      },
      { type: 'h2', text: 'Three Steps' },
      {
        type: 'orderedList',
        items: [
          'Pre-train the base model on large text',
          'Collect human preference comparisons',
          'Fine-tune the model with PPO against a reward model',
        ],
      },
      { type: 'h2', text: 'Why It Works' },
      {
        type: 'p',
        text: 'Human raters can express preferences ("A is better than B") more easily than they can write demonstrations. RLHF distills these preferences into a model.',
      },
    ]),
  });

  const finetuning = await createNode({
    type: 'document',
    title: 'Fine-tuning',
    emoji: '🔧',
    tags: ['training', 'adaptation'],
    collections: [foundations.id],
    template: 'book-note',
    properties: {
      Author: 'Various',
      Pages: '40',
      Rating: '4',
      Summary: 'Adapting a pre-trained model to a specific task or domain',
    },
    content: doc([
      { type: 'h1', text: 'Fine-tuning' },
      {
        type: 'p',
        text: 'Fine-tuning takes a pre-trained LLM and continues training it on a smaller, task-specific dataset. This adapts the model to a particular style, format, or domain.',
      },
      { type: 'h2', text: 'When to Fine-tune' },
      {
        type: 'bulletList',
        items: [
          'You need a specific output format that prompting alone cannot enforce',
          'You have a private knowledge base the base model does not know about',
          'Latency or cost rules out long prompts (consider RAG instead)',
        ],
      },
      { type: 'h2', text: 'Techniques' },
      {
        type: 'bulletList',
        items: [
          'Full fine-tuning — updates all weights, expensive',
          'LoRA — adds small trainable matrices, much cheaper',
          'QLoRA — combines LoRA with quantization',
        ],
      },
    ]),
  });

  const promptEng = await createNode({
    type: 'document',
    title: 'Prompt Engineering',
    emoji: '✍️',
    tags: ['applications', 'practical'],
    collections: [applications.id],
    template: 'book-note',
    properties: {
      Author: 'Multiple',
      Pages: '30',
      Rating: '4',
      Summary: 'Techniques for getting better outputs from LLMs',
    },
    content: doc([
      { type: 'h1', text: 'Prompt Engineering' },
      {
        type: 'p',
        text: 'Prompt engineering is the practice of crafting inputs to LLMs to get reliable, useful outputs.',
      },
      { type: 'h2', text: 'Core Techniques' },
      {
        type: 'bulletList',
        items: [
          'Be specific about role, format, and constraints',
          'Few-shot prompting — provide examples',
          'Chain-of-thought — ask the model to reason step by step',
          'Self-consistency — sample multiple times and vote',
        ],
      },
      { type: 'h2', text: 'Anti-Patterns' },
      {
        type: 'bulletList',
        items: [
          'Vague instructions ("write something good")',
          'Mixing multiple tasks in one prompt',
          'Not testing on edge cases',
        ],
      },
    ]),
  });

  // ---------------- People ----------------
  const hinton = await createNode({
    type: 'document',
    title: 'Geoffrey Hinton',
    emoji: '👤',
    tags: ['people', 'pioneer'],
    collections: [peopleCol.id],
    template: 'person',
    properties: {
      Role: 'Researcher',
      Organization: 'University of Toronto / Google',
      Email: 'hinton@cs.toronto.edu',
    },
    content: doc([
      { type: 'h1', text: 'Geoffrey Hinton' },
      {
        type: 'p',
        text: 'Often called the "Godfather of Deep Learning." Hinton pioneered backpropagation, Boltzmann machines, and capsule networks. He won the 2018 Turing Award alongside Yoshua Bengio and Yann LeCun.',
      },
      { type: 'h2', text: 'Key Contributions' },
      {
        type: 'bulletList',
        items: [
          'Backpropagation algorithm (1986)',
          'Boltzmann machines',
          'AlexNet (2012) with Alex Krizhevsky — kicked off the deep learning revolution',
        ],
      },
      {
        type: 'p',
        text: 'His work on Neural Networks laid the foundation for modern AI.',
      },
    ]),
  });

  const karpathy = await createNode({
    type: 'document',
    title: 'Andrej Karpathy',
    emoji: '👤',
    tags: ['people', 'educator'],
    collections: [peopleCol.id],
    template: 'person',
    properties: {
      Role: 'Engineer / Educator',
      Organization: 'Eureka Labs',
      Email: '',
    },
    content: doc([
      { type: 'h1', text: 'Andrej Karpathy' },
      {
        type: 'p',
        text: 'Former Director of AI at Tesla, founding member of OpenAI, and creator of one of the best free deep learning courses on the internet.',
      },
      { type: 'h2', text: 'Notable Work' },
      {
        type: 'bulletList',
        items: [
          'Stanford CS231n — convolutional neural networks',
          'nanoGPT — minimal GPT training codebase',
          'Tesla Autopilot vision system',
        ],
      },
      {
        type: 'p',
        text: 'Wrote the "A Recipe for Training Neural Networks" blog post. Great explainer of Transformers.',
      },
    ]),
  });

  // ---------------- Tasks ----------------
  const task1 = await createNode({
    type: 'task',
    title: 'Build internal chatbot',
    emoji: '💬',
    tags: ['project', 'priority'],
    taskStatus: 'in-progress',
    content: doc([
      { type: 'h1', text: 'Build internal chatbot' },
      {
        type: 'p',
        text: 'Build a RAG-based chatbot over our internal docs. Use RAG with embeddings stored in a vector DB.',
      },
      { type: 'h2', text: 'Steps' },
      {
        type: 'orderedList',
        items: [
          'Set up vector store',
          'Ingest documents',
          'Wire up the retrieval + LLM call',
          'Add evaluation harness',
        ],
      },
    ]),
  });

  const task2 = await createNode({
    type: 'task',
    title: 'Review "Attention Is All You Need"',
    emoji: '📄',
    tags: ['reading', 'learning'],
    taskStatus: 'pending',
    content: doc([
      { type: 'h1', text: 'Review "Attention Is All You Need"' },
      {
        type: 'p',
        text: 'Read and summarize the original transformer paper. Take notes on Transformers.',
      },
    ]),
  });

  const task3 = await createNode({
    type: 'task',
    title: 'Compare LoRA vs full fine-tuning',
    emoji: '🧪',
    tags: ['experiment'],
    taskStatus: 'completed',
    content: doc([
      { type: 'h1', text: 'LoRA vs full fine-tuning experiment' },
      {
        type: 'p',
        text: 'Run a small experiment comparing LoRA and full fine-tuning on a domain-specific classification task. See Fine-tuning.',
      },
    ]),
  });

  // ---------------- Project ----------------
  const project = await createNode({
    type: 'project',
    title: 'AI Research Hub',
    emoji: '🎓',
    tags: ['project', 'long-term'],
    properties: {
      Status: 'Active',
      Team: 'ML Platform',
      Started: '2026-01',
    },
    content: doc([
      { type: 'h1', text: 'AI Research Hub' },
      {
        type: 'p',
        text: "A long-running project to build up our team's AI/ML knowledge base and ship internal tools. Includes reading groups, paper reviews, and applied experiments.",
      },
      { type: 'h2', text: 'Goals' },
      {
        type: 'bulletList',
        items: [
          'Document core AI concepts in this workspace',
          'Build useful internal tools (see Build internal chatbot)',
          'Run quarterly experiments on Fine-tuning',
        ],
      },
      { type: 'h2', text: 'Members' },
      {
        type: 'p',
        text: 'Inspired by Geoffrey Hinton and Andrej Karpathy.',
      },
    ]),
  });

  // ---------------- Edges (typed relationships) ----------------
  await createEdge({ source: llm.id, target: transformers.id, type: 'extends' });
  await createEdge({ source: llm.id, target: nn.id, type: 'extends' });
  await createEdge({ source: rag.id, target: llm.id, type: 'uses' });
  await createEdge({ source: rag.id, target: embeddings.id, type: 'uses' });
  await createEdge({ source: finetuning.id, target: nn.id, type: 'extends' });
  await createEdge({ source: rlhf.id, target: finetuning.id, type: 'extends' });
  await createEdge({ source: promptEng.id, target: llm.id, type: 'uses' });
  await createEdge({ source: task1.id, target: rag.id, type: 'uses' });
  await createEdge({ source: task1.id, target: embeddings.id, type: 'uses' });
  await createEdge({ source: task2.id, target: transformers.id, type: 'related-to' });
  await createEdge({ source: task3.id, target: finetuning.id, type: 'related-to' });
  await createEdge({ source: project.id, target: task1.id, type: 'project-member' });
  await createEdge({ source: project.id, target: task3.id, type: 'project-member' });
  await createEdge({ source: hinton.id, target: nn.id, type: 'created-by' });
  await createEdge({ source: karpathy.id, target: transformers.id, type: 'related-to' });
  await createEdge({ source: llm.id, target: finetuning.id, type: 'related-to' });
  await createEdge({ source: rag.id, target: promptEng.id, type: 'related-to' });

  // ---------------- Revisions for Transformers ----------------
  const transformersOriginal = doc([
    { type: 'h1', text: 'Transformers (draft)' },
    { type: 'p', text: 'First draft — short notes on self-attention.' },
  ]);
  const transformersV2 = doc([
    { type: 'h1', text: 'Transformers' },
    {
      type: 'p',
      text: 'Expanded draft covering multi-head attention and positional encodings.',
    },
    { type: 'h2', text: 'Self-Attention' },
    { type: 'p', text: 'Each token attends to every other token in the sequence.' },
  ]);

  await createRevision({
    nodeId: transformers.id,
    content: transformersOriginal,
    plainText: extractTextFromTiptap(transformersOriginal),
  });
  const firstRevs = await db.revisions
    .where('nodeId')
    .equals(transformers.id)
    .reverse()
    .sortBy('createdAt');
  if (firstRevs.length > 0) {
    await db.revisions.update(firstRevs[0].id, {
      createdAt: Date.now() - 1000 * 60 * 60 * 24 * 3,
    });
  }

  await createRevision({
    nodeId: transformers.id,
    content: transformersV2,
    plainText: extractTextFromTiptap(transformersV2),
  });
  const secondRevs = await db.revisions
    .where('nodeId')
    .equals(transformers.id)
    .reverse()
    .sortBy('createdAt');
  if (secondRevs.length > 0) {
    await db.revisions.update(secondRevs[0].id, {
      createdAt: Date.now() - 1000 * 60 * 60 * 24,
    });
  }

  // Update plain text on all nodes (createNode defaults to '')
  const allNodes = await db.nodes.toArray();
  for (const n of allNodes) {
    if (n.content) {
      await db.nodes.update(n.id, {
        plainText: extractTextFromTiptap(n.content as JSONContent),
      });
    }
  }

  // eslint-disable-next-line no-console
  console.log('Demo data seeded:', {
    nodes: await db.nodes.count(),
    edges: await db.edges.count(),
    collections: await db.collections.count(),
    revisions: await db.revisions.count(),
  });
}
