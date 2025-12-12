import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Box,
  Container,
  Heading,
  VStack,
  HStack,
  Text,
  Input,
  Button,
  Card,
  CardBody,
  Tabs,
  TabList,
  TabPanels,
  Tab,
  TabPanel,
  Badge,
  Link,
  Switch,
  FormControl,
  FormLabel,
  Select,
  useToast,
} from "@chakra-ui/react";

/// Settings page - configure API keys and services

interface ApiKeyStatus {
  has_key: boolean;
  masked_key?: string;
}

interface ServiceConfig {
  id?: number;
  service_type: string;
  provider: string;
  is_active: boolean;
  settings?: string;
  has_api_key: boolean;
}

// ASR Provider configurations
const ASR_PROVIDERS = {
  assemblyai: {
    name: "AssemblyAI",
    signupUrl: "https://www.assemblyai.com/",
  },
  deepgram: {
    name: "Deepgram",
    signupUrl: "https://deepgram.com/",
  },
};

// LLM Provider configurations (models are fetched dynamically from API)
const LLM_PROVIDERS = {
  openai: {
    name: "OpenAI",
    signupUrl: "https://platform.openai.com/",
  },
  anthropic: {
    name: "Anthropic (Claude)",
    signupUrl: "https://console.anthropic.com/",
  },
  google: {
    name: "Google (Gemini)",
    signupUrl: "https://makersuite.google.com/app/apikey",
  },
  groq: {
    name: "Groq",
    signupUrl: "https://console.groq.com/",
  },
};

interface ModelInfo {
  id: string;
  name: string;
  provider: string;
  context_window: number;
  is_fallback_context_window?: boolean;
}

function Settings() {
  const toast = useToast();

  // ASR state
  const [asrConfigs, setAsrConfigs] = useState<Record<string, ServiceConfig>>({});
  const [asrApiKeys, setAsrApiKeys] = useState<Record<string, string>>({});
  const [asrKeyStatuses, setAsrKeyStatuses] = useState<Record<string, ApiKeyStatus>>({});
  const [asrModels, setAsrModels] = useState<Record<string, string>>({});
  const [asrAvailableModels, setAsrAvailableModels] = useState<Record<string, any[]>>({});
  const [asrModelsLoading, setAsrModelsLoading] = useState<Record<string, boolean>>({});
  const [asrModelChanged, setAsrModelChanged] = useState<Record<string, boolean>>({});

  // LLM state
  const [llmConfigs, setLlmConfigs] = useState<Record<string, ServiceConfig>>({});
  const [llmApiKeys, setLlmApiKeys] = useState<Record<string, string>>({});
  const [llmKeyStatuses, setLlmKeyStatuses] = useState<Record<string, ApiKeyStatus>>({});
  const [llmModels, setLlmModels] = useState<Record<string, string>>({});
  const [llmAvailableModels, setLlmAvailableModels] = useState<Record<string, ModelInfo[]>>({});
  const [llmModelsLoading, setLlmModelsLoading] = useState<Record<string, boolean>>({});
  const [llmModelChanged, setLlmModelChanged] = useState<Record<string, boolean>>({});

  // Loading states
  const [loading, setLoading] = useState<Record<string, boolean>>({});

  // Load all configurations on mount
  useEffect(() => {
    loadAllConfigs();
  }, []);

  const loadAllConfigs = async () => {
    try {
      // Load ASR configs
      for (const provider of Object.keys(ASR_PROVIDERS)) {
        await loadConfig("asr", provider);
      }

      // Load LLM configs
      for (const provider of Object.keys(LLM_PROVIDERS)) {
        await loadConfig("llm", provider);
      }
    } catch (err) {
      console.error("Error loading configs:", err);
      toast({
        title: "Error loading configurations",
        description: String(err),
        status: "error",
        duration: 5000,
        isClosable: true,
      });
    }
  };

  const fetchLlmModels = async (provider: string) => {
    setLlmModelsLoading((prev) => ({ ...prev, [provider]: true }));
    try {
      const response = await invoke<{ models: ModelInfo[] }>("fetch_llm_models", {
        request: { provider },
      });
      setLlmAvailableModels((prev) => ({ ...prev, [provider]: response.models }));
      console.log(`Fetched ${response.models.length} models for ${provider}`);
    } catch (err) {
      console.error(`Failed to fetch models for ${provider}:`, err);
      // Don't show error to user - just keep empty models list
    } finally {
      setLlmModelsLoading((prev) => ({ ...prev, [provider]: false }));
    }
  };

  const fetchAsrModels = async (provider: string) => {
    setAsrModelsLoading((prev) => ({ ...prev, [provider]: true }));
    try {
      const models = await invoke<any[]>("fetch_asr_models", {
        provider,
      });
      setAsrAvailableModels((prev) => ({ ...prev, [provider]: models }));
      console.log(`Fetched ${models.length} models for ${provider}`);
    } catch (err) {
      console.error(`Failed to fetch ASR models for ${provider}:`, err);
      // Don't show error to user - just keep empty models list
    } finally {
      setAsrModelsLoading((prev) => ({ ...prev, [provider]: false }));
    }
  };

  const loadConfig = async (serviceType: string, provider: string) => {
    try {
      // Load API key status
      const keyStatus = await invoke<ApiKeyStatus>("get_api_key_status", {
        request: { service_type: serviceType, provider },
      });

      // Load service configuration
      const config = await invoke<ServiceConfig | null>("get_service_config", {
        serviceType,
        provider,
      });

      // Parse settings to get model
      let model = "";
      if (config?.settings) {
        try {
          const settings = JSON.parse(config.settings);
          model = settings.model || "";
        } catch (e) {
          console.error("Error parsing settings:", e);
        }
      }

      // Update state
      if (serviceType === "asr") {
        setAsrKeyStatuses((prev) => ({ ...prev, [provider]: keyStatus }));
        if (config) {
          setAsrConfigs((prev) => ({ ...prev, [provider]: config }));
          setAsrModels((prev) => ({ ...prev, [provider]: model }));
        }
        // Fetch available models if API key exists
        if (keyStatus.has_key) {
          fetchAsrModels(provider);
        }
      } else {
        setLlmKeyStatuses((prev) => ({ ...prev, [provider]: keyStatus }));
        if (config) {
          setLlmConfigs((prev) => ({ ...prev, [provider]: config }));
          setLlmModels((prev) => ({ ...prev, [provider]: model }));
        }
        // Fetch available models if API key exists
        if (keyStatus.has_key) {
          fetchLlmModels(provider);
        }
      }
    } catch (err) {
      console.error(`Error loading config for ${serviceType}:${provider}:`, err);
    }
  };

  const handleSaveApiKey = async (serviceType: string, provider: string) => {
    const key = serviceType === "asr" ? asrApiKeys[provider] : llmApiKeys[provider];

    if (!key || key.trim() === "") {
      toast({
        title: "Error",
        description: "Please enter an API key",
        status: "error",
        duration: 3000,
        isClosable: true,
      });
      return;
    }

    setLoading((prev) => ({ ...prev, [`${serviceType}_${provider}`]: true }));

    try {
      // Save API key to keychain
      await invoke("save_api_key", {
        request: {
          service_type: serviceType,
          provider,
          api_key: key,
        },
      });

      // Clear the input
      if (serviceType === "asr") {
        setAsrApiKeys((prev) => ({ ...prev, [provider]: "" }));
      } else {
        setLlmApiKeys((prev) => ({ ...prev, [provider]: "" }));
      }

      // Reload config to update status
      await loadConfig(serviceType, provider);

      toast({
        title: "Success",
        description: `API key saved successfully for ${provider}`,
        status: "success",
        duration: 3000,
        isClosable: true,
      });
    } catch (err) {
      toast({
        title: "Error",
        description: `Failed to save API key: ${err}`,
        status: "error",
        duration: 5000,
        isClosable: true,
      });
    } finally {
      setLoading((prev) => ({ ...prev, [`${serviceType}_${provider}`]: false }));
    }
  };

  const handleSaveModel = async (serviceType: string, provider: string) => {
    const model = serviceType === "asr" ? asrModels[provider] : llmModels[provider];

    setLoading((prev) => ({ ...prev, [`model_${serviceType}_${provider}`]: true }));

    try {
      // Create settings JSON
      const settings = JSON.stringify({ model });

      // Save configuration with is_active = true to automatically activate when model is selected
      await invoke("save_service_config", {
        request: {
          service_type: serviceType,
          provider,
          is_active: true, // Auto-activate when model is selected
          settings,
        },
      });

      // Reload all configs of this service type to update active status across all providers
      if (serviceType === "asr") {
        for (const p of Object.keys(ASR_PROVIDERS)) {
          await loadConfig("asr", p);
        }
        setAsrModelChanged((prev) => ({ ...prev, [provider]: false }));
      } else {
        for (const p of Object.keys(LLM_PROVIDERS)) {
          await loadConfig("llm", p);
        }
        setLlmModelChanged((prev) => ({ ...prev, [provider]: false }));
      }

      toast({
        title: "Success",
        description: `${provider} activated with selected model`,
        status: "success",
        duration: 3000,
        isClosable: true,
      });
    } catch (err) {
      toast({
        title: "Error",
        description: `Failed to save model: ${err}`,
        status: "error",
        duration: 5000,
        isClosable: true,
      });
    } finally {
      setLoading((prev) => ({ ...prev, [`model_${serviceType}_${provider}`]: false }));
    }
  };

  const handleToggleService = async (serviceType: string, provider: string, activate: boolean) => {
    const loadingKey = activate ? `activate_${serviceType}_${provider}` : `deactivate_${serviceType}_${provider}`;
    setLoading((prev) => ({ ...prev, [loadingKey]: true }));

    try {
      if (activate) {
        await invoke("activate_service", {
          serviceType,
          provider,
        });
      } else {
        // Get current config
        const configs = serviceType === "asr" ? asrConfigs : llmConfigs;
        const existingConfig = configs[provider];

        // Save with is_active = false
        await invoke("save_service_config", {
          request: {
            service_type: serviceType,
            provider,
            is_active: false,
            settings: existingConfig?.settings || null,
          },
        });
      }

      // Reload all configs of this service type
      if (serviceType === "asr") {
        for (const p of Object.keys(ASR_PROVIDERS)) {
          await loadConfig("asr", p);
        }
      } else {
        for (const p of Object.keys(LLM_PROVIDERS)) {
          await loadConfig("llm", p);
        }
      }

      toast({
        title: "Success",
        description: `${provider} ${activate ? "activated" : "deactivated"} successfully`,
        status: "success",
        duration: 3000,
        isClosable: true,
      });
    } catch (err) {
      toast({
        title: "Error",
        description: `Failed to ${activate ? "activate" : "deactivate"} service: ${err}`,
        status: "error",
        duration: 5000,
        isClosable: true,
      });
    } finally {
      setLoading((prev) => ({ ...prev, [loadingKey]: false }));
    }
  };

  const handleDeleteApiKey = async (serviceType: string, provider: string) => {
    if (!confirm(`Are you sure you want to delete the API key for ${provider}?`)) {
      return;
    }

    setLoading((prev) => ({ ...prev, [`delete_${serviceType}_${provider}`]: true }));

    try {
      await invoke("delete_api_key", {
        serviceType,
        provider,
      });

      // Reload config
      await loadConfig(serviceType, provider);

      toast({
        title: "Success",
        description: `API key deleted for ${provider}`,
        status: "success",
        duration: 3000,
        isClosable: true,
      });
    } catch (err) {
      toast({
        title: "Error",
        description: `Failed to delete API key: ${err}`,
        status: "error",
        duration: 5000,
        isClosable: true,
      });
    } finally {
      setLoading((prev) => ({ ...prev, [`delete_${serviceType}_${provider}`]: false }));
    }
  };

  const renderServiceCard = (
    serviceType: string,
    provider: string,
    providerConfig: { name: string; signupUrl: string }
  ) => {
    const configs = serviceType === "asr" ? asrConfigs : llmConfigs;
    const keyStatuses = serviceType === "asr" ? asrKeyStatuses : llmKeyStatuses;
    const apiKeys = serviceType === "asr" ? asrApiKeys : llmApiKeys;
    const models = serviceType === "asr" ? asrModels : llmModels;
    const availableModels = serviceType === "asr" ? asrAvailableModels : llmAvailableModels;
    const modelsLoading = serviceType === "asr" ? asrModelsLoading : llmModelsLoading;
    const modelChanged = serviceType === "asr" ? asrModelChanged : llmModelChanged;

    const config = configs[provider];
    const keyStatus = keyStatuses[provider];
    const apiKey = apiKeys[provider] || "";
    const selectedModel = models[provider] || "";

    const hasKey = keyStatus?.has_key || false;
    const isActive = config?.is_active || false;
    const isLoading = loading[`${serviceType}_${provider}`] || false;
    const hasModelChanged = modelChanged[provider] || false;

    // Get saved model from config
    let savedModel = "";
    if (config?.settings) {
      try {
        const settings = JSON.parse(config.settings);
        savedModel = settings.model || "";
      } catch (e) {
        console.error("Error parsing settings:", e);
      }
    }

    return (
      <Card
        key={provider}
        variant="outline"
        borderWidth={isActive ? "2px" : "1px"}
        borderColor={isActive ? "green.500" : "gray.200"}
        bg={isActive ? "green.50" : "white"}
      >
        <CardBody>
          <VStack align="stretch" spacing={4}>
            {/* Header */}
            <HStack justify="space-between" align="center">
              <Heading size="md">{providerConfig.name}</Heading>
              <HStack>
                {isActive && (
                  <Badge colorScheme="green" fontSize="xs" px={3} py={1}>
                    ACTIVE
                  </Badge>
                )}
                {hasKey && savedModel && (
                  <Switch
                    isChecked={isActive}
                    onChange={(e) => handleToggleService(serviceType, provider, e.target.checked)}
                    isDisabled={loading[`activate_${serviceType}_${provider}`] || loading[`deactivate_${serviceType}_${provider}`]}
                    colorScheme="green"
                  />
                )}
              </HStack>
            </HStack>

            <Text fontSize="sm" color="gray.600">
              <Link href={providerConfig.signupUrl} isExternal color="blue.500">
                Sign up for {providerConfig.name}
              </Link>
            </Text>

            {/* API Key Section */}
            <FormControl>
              <FormLabel fontWeight="bold">API Key</FormLabel>
              {hasKey ? (
                <HStack>
                  <Box flex={1} p={2} bg="gray.100" borderRadius="md" fontFamily="monospace" fontSize="sm">
                    {keyStatus.masked_key}
                  </Box>
                  <Button
                    size="sm"
                    colorScheme="red"
                    onClick={() => handleDeleteApiKey(serviceType, provider)}
                    isLoading={loading[`delete_${serviceType}_${provider}`]}
                  >
                    Delete
                  </Button>
                </HStack>
              ) : (
                <HStack>
                  <Input
                    type="password"
                    placeholder="Enter API Key"
                    value={apiKey}
                    onChange={(e) => {
                      if (serviceType === "asr") {
                        setAsrApiKeys((prev) => ({ ...prev, [provider]: e.target.value }));
                      } else {
                        setLlmApiKeys((prev) => ({ ...prev, [provider]: e.target.value }));
                      }
                    }}
                  />
                  <Button
                    colorScheme="blue"
                    onClick={() => handleSaveApiKey(serviceType, provider)}
                    isLoading={isLoading}
                  >
                    Save
                  </Button>
                </HStack>
              )}
            </FormControl>

            {/* Model Selection */}
            {hasKey && (
              <FormControl>
                <FormLabel fontWeight="bold">
                  Model
                  {modelsLoading[provider] && (
                    <Text as="span" fontWeight="normal" ml={2} color="gray.500">
                      (Loading models...)
                    </Text>
                  )}
                </FormLabel>
                <VStack align="stretch" spacing={2}>
                  <Select
                    placeholder="Select a model"
                    value={selectedModel}
                    onChange={(e) => {
                      const model = e.target.value;
                      if (serviceType === "asr") {
                        setAsrModels((prev) => ({ ...prev, [provider]: model }));
                        setAsrModelChanged((prev) => ({ ...prev, [provider]: model !== savedModel }));
                      } else {
                        setLlmModels((prev) => ({ ...prev, [provider]: model }));
                        setLlmModelChanged((prev) => ({ ...prev, [provider]: model !== savedModel }));
                      }
                    }}
                    isDisabled={modelsLoading[provider]}
                  >
                    {serviceType === "asr" && availableModels[provider]?.map((model) => {
                      const modelId = model.canonical_name || model.id || model.name;
                      const modelName = model.name || model.id;
                      const modelVersion = model.version ? ` (${model.version})` : "";
                      const modelDesc = model.description ? ` - ${model.description}` : "";
                      const displayName = `${modelName}${modelVersion}${modelDesc}`;

                      return (
                        <option key={modelId} value={modelId}>
                          {displayName}
                        </option>
                      );
                    })}
                    {serviceType === "llm" &&
                      availableModels[provider]
                        ?.slice()
                        .sort((a, b) => a.name.localeCompare(b.name))
                        .map((model) => (
                          <option
                            key={model.id}
                            value={model.id}
                            title={`Context: ${model.context_window.toLocaleString()} tokens`}
                          >
                            {model.name} ({model.context_window.toLocaleString()} tokens)
                            {model.is_fallback_context_window && " ⚠️"}
                          </option>
                        ))}
                  </Select>

                  {hasModelChanged && selectedModel && (
                    <Button
                      size="sm"
                      colorScheme="blue"
                      onClick={() => handleSaveModel(serviceType, provider)}
                      isLoading={loading[`model_${serviceType}_${provider}`]}
                    >
                      Save Model Selection
                    </Button>
                  )}

                  {availableModels[provider]?.length === 0 && !modelsLoading[provider] && (
                    <Text fontSize="sm" color="gray.500">
                      No models available. Check your API key or try saving it again.
                    </Text>
                  )}

                  {serviceType === "llm" && (
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => fetchLlmModels(provider)}
                      isLoading={llmModelsLoading[provider]}
                    >
                      Refresh Models
                    </Button>
                  )}
                </VStack>
              </FormControl>
            )}

            {/* Status Message */}
            {hasKey && savedModel && (
              <Box p={3} bg="blue.50" borderRadius="md" fontSize="sm">
                {isActive ? (
                  <Text color="blue.700" fontWeight="bold">
                    ✓ This service is currently active and will be used for all{" "}
                    {serviceType === "asr" ? "transcriptions" : "insights"}
                  </Text>
                ) : (
                  <Text color="gray.600">
                    💡 Use the toggle switch above to activate this service
                  </Text>
                )}
              </Box>
            )}
          </VStack>
        </CardBody>
      </Card>
    );
  };

  return (
    <Container maxW="container.xl" py={8}>
      <VStack align="stretch" spacing={6}>
        <Box>
          <Heading size="lg" mb={2}>
            Settings
          </Heading>
          <Text color="gray.600">
            Configure your ASR and LLM services with API keys stored securely in your OS keychain.
          </Text>
        </Box>

        <Tabs variant="enclosed" colorScheme="blue">
          <TabList>
            <Tab>Transcription Services (ASR)</Tab>
            <Tab>AI Analysis (LLM)</Tab>
          </TabList>

          <TabPanels>
            {/* ASR Services Tab */}
            <TabPanel>
              <VStack align="stretch" spacing={4}>
                <Box>
                  <Heading size="md" mb={2}>
                    Speech Recognition Services
                  </Heading>
                  <Text fontSize="sm" color="gray.600">
                    Configure your speech recognition service. Only one ASR service can be active at a time.
                  </Text>
                </Box>

                {Object.entries(ASR_PROVIDERS).map(([provider, config]) =>
                  renderServiceCard("asr", provider, config)
                )}
              </VStack>
            </TabPanel>

            {/* LLM Services Tab */}
            <TabPanel>
              <VStack align="stretch" spacing={4}>
                <Box>
                  <Heading size="md" mb={2}>
                    Language Model Services
                  </Heading>
                  <Text fontSize="sm" color="gray.600">
                    Configure your language model service. Only one LLM service can be active at a time.
                  </Text>
                </Box>

                {Object.entries(LLM_PROVIDERS).map(([provider, config]) =>
                  renderServiceCard("llm", provider, config)
                )}
              </VStack>
            </TabPanel>
          </TabPanels>
        </Tabs>

        {/* Security Note */}
        <Box p={4} bg="orange.50" borderRadius="md">
          <Heading size="sm" mb={2}>
            🔒 Security
          </Heading>
          <Text fontSize="sm" color="gray.700">
            Your API keys are stored securely in your operating system's keychain (Windows Credential Manager on
            Windows, Secret Service on Linux). They are never stored in plain text or in the database.
          </Text>
        </Box>
      </VStack>
    </Container>
  );
}

export default Settings;
